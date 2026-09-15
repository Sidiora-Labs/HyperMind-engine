#![allow(clippy::missing_errors_doc)]

use crate::frame::FrameHeader;
use hm_core::{Error, ErrorCode};
use std::collections::{BTreeMap, BTreeSet};

pub type Hash = [u8; 32];

const NODE_DOMAIN: u8 = 0x01;
const ROOT_DOMAIN: u8 = 0x02;
const FRAME_DOMAIN: u8 = 0x03;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Node {
    pub height: u8,
    pub start: u64,
    pub leaf_count: u64,
    pub hash: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendResult {
    pub root: Hash,
    pub created_nodes: Vec<Node>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeProof {
    pub total_leaf_count: u64,
    pub range_start: u64,
    pub range_leaf_count: u64,
    pub expected_root: Hash,
    pub boundary_nodes: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct Mmr {
    leaves: Vec<Hash>,
    peaks: Vec<Node>,
    nodes: BTreeMap<(u64, u64), Node>,
    leaf_count: u64,
    retain_history: bool,
}

impl Default for Mmr {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Mmr {
    #[must_use]
    pub const fn new(retain_history: bool) -> Self {
        Self {
            leaves: Vec::new(),
            peaks: Vec::new(),
            nodes: BTreeMap::new(),
            leaf_count: 0,
            retain_history,
        }
    }

    #[must_use]
    pub fn append(&mut self, leaf_hash: Hash) -> AppendResult {
        let mut node = Node {
            height: 0,
            start: self.leaf_count,
            leaf_count: 1,
            hash: leaf_hash,
        };
        if self.retain_history {
            self.leaves.push(leaf_hash);
        }
        self.leaf_count += 1;
        let mut created_nodes = vec![node];
        if self.retain_history {
            self.nodes.insert((node.start, node.leaf_count), node);
        }
        while self
            .peaks
            .last()
            .is_some_and(|peak| peak.height == node.height)
        {
            let Some(left) = self.peaks.pop() else {
                break;
            };
            node = Node {
                height: node.height + 1,
                start: left.start,
                leaf_count: left.leaf_count + node.leaf_count,
                hash: parent_hash(&left.hash, &node.hash),
            };
            if self.retain_history {
                self.nodes.insert((node.start, node.leaf_count), node);
            }
            created_nodes.push(node);
        }
        self.peaks.push(node);
        AppendResult {
            root: self.root(),
            created_nodes,
        }
    }

    #[must_use]
    pub fn root(&self) -> Hash {
        root_from_peaks(self.leaf_count, &self.peaks)
    }

    pub fn root_at(&self, leaf_count: u64) -> Result<Hash, Error> {
        if leaf_count > self.leaf_count {
            return Err(Error::new(ErrorCode::InvalidArgument).at_offset(leaf_count));
        }
        let mut peaks = Vec::new();
        let mut start = 0;
        let mut remaining = leaf_count;
        while remaining > 0 {
            let size = highest_power_of_two(remaining);
            peaks.push(self.lookup(start, size)?);
            start += size;
            remaining -= size;
        }
        Ok(root_from_peaks(leaf_count, &peaks))
    }

    pub fn prove_range(
        &self,
        range_start: u64,
        range_leaf_count: u64,
    ) -> Result<RangeProof, Error> {
        if range_leaf_count == 0
            || range_start >= self.leaf_count
            || range_leaf_count > self.leaf_count - range_start
        {
            return Err(Error::new(ErrorCode::InvalidArgument).at_offset(range_start));
        }
        let mut proof = RangeProof {
            total_leaf_count: self.leaf_count,
            range_start,
            range_leaf_count,
            expected_root: self.root(),
            boundary_nodes: Vec::new(),
        };
        for peak in &self.peaks {
            self.collect_boundary(*peak, range_start, range_leaf_count, &mut proof)?;
        }
        Ok(proof)
    }

    pub fn verify_range(range_leaf_hashes: &[Hash], proof: &RangeProof) -> Result<Hash, Error> {
        let supplied = u64::try_from(range_leaf_hashes.len())
            .map_err(|_| Error::new(ErrorCode::ProofInvalid))?;
        if proof.range_leaf_count == 0
            || supplied != proof.range_leaf_count
            || proof.range_start >= proof.total_leaf_count
            || proof.range_leaf_count > proof.total_leaf_count - proof.range_start
        {
            return Err(Error::new(ErrorCode::ProofInvalid).at_offset(proof.range_start));
        }
        let mut boundary = BTreeMap::new();
        for node in &proof.boundary_nodes {
            if !node.leaf_count.is_power_of_two()
                || node.height != height_for(node.leaf_count)
                || node.start % node.leaf_count != 0
                || !disjoint(
                    node.start,
                    node.leaf_count,
                    proof.range_start,
                    proof.range_leaf_count,
                )
                || node.start > proof.total_leaf_count
                || node.leaf_count > proof.total_leaf_count - node.start
                || boundary
                    .insert((node.start, node.leaf_count), *node)
                    .is_some()
            {
                return Err(Error::new(ErrorCode::ProofInvalid).at_offset(node.start));
            }
        }
        let mut used = BTreeSet::new();
        let mut peaks = Vec::new();
        let mut start = 0;
        let mut remaining = proof.total_leaf_count;
        while remaining > 0 {
            let count = highest_power_of_two(remaining);
            peaks.push(rebuild_range(
                start,
                count,
                range_leaf_hashes,
                proof,
                &boundary,
                &mut used,
            )?);
            start += count;
            remaining -= count;
        }
        if used.len() != boundary.len() {
            return Err(Error::new(ErrorCode::ProofInvalid));
        }
        let actual = root_from_peaks(proof.total_leaf_count, &peaks);
        if actual != proof.expected_root {
            return Err(Error::new(ErrorCode::ProofInvalid));
        }
        Ok(actual)
    }

    #[must_use]
    pub const fn leaf_count(&self) -> u64 {
        self.leaf_count
    }

    #[must_use]
    pub fn leaves(&self) -> &[Hash] {
        &self.leaves
    }

    fn lookup(&self, start: u64, leaf_count: u64) -> Result<Node, Error> {
        self.nodes
            .get(&(start, leaf_count))
            .copied()
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation).at_offset(start))
    }

    fn collect_boundary(
        &self,
        node: Node,
        range_start: u64,
        range_count: u64,
        proof: &mut RangeProof,
    ) -> Result<(), Error> {
        if disjoint(node.start, node.leaf_count, range_start, range_count) {
            proof.boundary_nodes.push(node);
            return Ok(());
        }
        if inside(node.start, node.leaf_count, range_start, range_count) {
            return Ok(());
        }
        if node.leaf_count == 1 {
            return Err(Error::new(ErrorCode::InvariantViolation).at_offset(node.start));
        }
        let child_count = node.leaf_count / 2;
        self.collect_boundary(
            self.lookup(node.start, child_count)?,
            range_start,
            range_count,
            proof,
        )?;
        self.collect_boundary(
            self.lookup(node.start + child_count, child_count)?,
            range_start,
            range_count,
            proof,
        )
    }
}

#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> Hash {
    *blake3::hash(bytes).as_bytes()
}

#[must_use]
pub fn hash_frame_sealed(header: &FrameHeader, sealed_payload: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[FRAME_DOMAIN]);
    hasher.update(&header.lsn.get().to_le_bytes());
    hasher.update(&[header.kind as u8]);
    hasher.update(&header.wall_timestamp_ns.get().cast_unsigned().to_le_bytes());
    hasher.update(&header.actor.get().to_le_bytes());
    hasher.update(header.conversation.as_bytes());
    hasher.update(&(sealed_payload.len() as u64).to_le_bytes());
    hasher.update(sealed_payload);
    *hasher.finalize().as_bytes()
}

#[must_use]
pub fn root_from_peaks(leaf_count: u64, peaks: &[Node]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[ROOT_DOMAIN]);
    hasher.update(&leaf_count.to_le_bytes());
    for peak in peaks {
        hasher.update(&peak.hash);
    }
    *hasher.finalize().as_bytes()
}

fn parent_hash(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[NODE_DOMAIN]);
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

fn highest_power_of_two(value: u64) -> u64 {
    1_u64 << (u64::BITS - value.leading_zeros() - 1)
}

fn height_for(leaf_count: u64) -> u8 {
    u8::try_from(leaf_count.trailing_zeros()).expect("u64 tree height fits u8")
}

fn disjoint(first_start: u64, first_count: u64, second_start: u64, second_count: u64) -> bool {
    first_start.saturating_add(first_count) <= second_start
        || second_start.saturating_add(second_count) <= first_start
}

fn inside(node_start: u64, node_count: u64, range_start: u64, range_count: u64) -> bool {
    node_start >= range_start
        && node_start.saturating_add(node_count) <= range_start.saturating_add(range_count)
}

fn rebuild_range(
    start: u64,
    count: u64,
    hashes: &[Hash],
    proof: &RangeProof,
    boundary: &BTreeMap<(u64, u64), Node>,
    used: &mut BTreeSet<(u64, u64)>,
) -> Result<Node, Error> {
    if disjoint(start, count, proof.range_start, proof.range_leaf_count) {
        let key = (start, count);
        let node = boundary
            .get(&key)
            .copied()
            .ok_or_else(|| Error::new(ErrorCode::ProofInvalid).at_offset(start))?;
        used.insert(key);
        return Ok(node);
    }
    if count == 1 {
        let index = start
            .checked_sub(proof.range_start)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|index| *index < hashes.len())
            .ok_or_else(|| Error::new(ErrorCode::ProofInvalid).at_offset(start))?;
        return Ok(Node {
            height: 0,
            start,
            leaf_count: 1,
            hash: hashes[index],
        });
    }
    let child_count = count / 2;
    let left = rebuild_range(start, child_count, hashes, proof, boundary, used)?;
    let right = rebuild_range(
        start + child_count,
        child_count,
        hashes,
        proof,
        boundary,
        used,
    )?;
    Ok(Node {
        height: left.height + 1,
        start,
        leaf_count: count,
        hash: parent_hash(&left.hash, &right.hash),
    })
}
