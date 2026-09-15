#![allow(clippy::missing_errors_doc)]

use crate::vectors::VectorLane;
use hm_core::{Error, ErrorCode};
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

const SPACE_MAGIC: &[u8; 8] = b"HMSPC001";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpaceDefinition {
    pub generation_id: String,
    pub space_id: String,
    pub encoder_id: String,
    pub revision: String,
    pub dimensions: usize,
    pub distance: String,
    pub normalization: String,
    pub input_role: String,
}

impl SpaceDefinition {
    #[must_use]
    pub fn compatible_with(&self, other: &Self) -> bool {
        self.dimensions == other.dimensions
            && self.distance == other.distance
            && self.normalization == other.normalization
            && self.input_role == other.input_role
    }
}

#[derive(Clone, Debug)]
pub struct SpaceCatalog {
    root: PathBuf,
}

impl SpaceCatalog {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, Error> {
        fs::create_dir_all(root.as_ref()).map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
        })
    }

    pub fn create(&self, definition: &SpaceDefinition) -> Result<(), Error> {
        validate_definition(definition)?;
        let bytes = encode_definition(definition)?;
        let path = self.metadata_path(&definition.generation_id);
        if path.exists() {
            return if fs::read(path).map_err(|_| Error::new(ErrorCode::ReadFailed))? == bytes {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::AlreadyExists))
            };
        }
        write_new(&path, &bytes)?;
        sync_directory(&self.root)
    }

    pub fn validate_generation(
        &self,
        generation_id: &str,
        lane: &VectorLane,
    ) -> Result<usize, Error> {
        let definition = self.definition(generation_id)?;
        if lane.generation_id() != definition.generation_id
            || lane.space_id() != definition.space_id
            || lane.dimensions() != definition.dimensions
        {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let records = lane.validate()?;
        if records == 0 {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let bytes = lane.canonical_bytes()?;
        write_replace(
            &self.validation_path(generation_id),
            blake3::hash(&bytes).as_bytes(),
        )?;
        sync_directory(&self.root)?;
        Ok(records)
    }

    pub fn activate(&self, generation_id: &str, lane: &VectorLane) -> Result<(), Error> {
        let definition = self.definition(generation_id)?;
        if lane.generation_id() != generation_id
            || lane.space_id() != definition.space_id
            || lane.dimensions() != definition.dimensions
        {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let expected = fs::read(self.validation_path(generation_id))
            .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
        if expected.as_slice() != blake3::hash(&lane.canonical_bytes()?).as_bytes() {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        if let Some(active) = self.active()? {
            let previous = self.definition(&active)?;
            if definition.compatible_with(&previous) {
                write_replace(&self.root.join("previous-compatible"), active.as_bytes())?;
            }
        }
        write_replace(&self.root.join("active"), generation_id.as_bytes())?;
        sync_directory(&self.root)
    }

    pub fn active(&self) -> Result<Option<String>, Error> {
        read_pointer(&self.root.join("active"))
    }

    pub fn previous_compatible(&self) -> Result<Option<String>, Error> {
        read_pointer(&self.root.join("previous-compatible"))
    }

    pub fn definition(&self, generation_id: &str) -> Result<SpaceDefinition, Error> {
        if generation_id.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let bytes = fs::read(self.metadata_path(generation_id))
            .map_err(|_| Error::new(ErrorCode::ReadFailed))?;
        let definition = decode_definition(&bytes)?;
        if definition.generation_id != generation_id {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        Ok(definition)
    }

    fn metadata_path(&self, generation_id: &str) -> PathBuf {
        self.root.join(format!(
            "{}.space",
            blake3::hash(generation_id.as_bytes()).to_hex()
        ))
    }

    fn validation_path(&self, generation_id: &str) -> PathBuf {
        self.root.join(format!(
            "{}.validated",
            blake3::hash(generation_id.as_bytes()).to_hex()
        ))
    }
}

fn validate_definition(definition: &SpaceDefinition) -> Result<(), Error> {
    let fields = [
        definition.generation_id.as_str(),
        definition.space_id.as_str(),
        definition.encoder_id.as_str(),
        definition.revision.as_str(),
        definition.distance.as_str(),
        definition.normalization.as_str(),
        definition.input_role.as_str(),
    ];
    if definition.dimensions == 0
        || fields
            .iter()
            .any(|field| field.is_empty() || field.len() > 4096)
    {
        Err(Error::new(ErrorCode::InvalidArgument))
    } else {
        Ok(())
    }
}

fn encode_definition(definition: &SpaceDefinition) -> Result<Vec<u8>, Error> {
    let dimensions = u32::try_from(definition.dimensions)
        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(SPACE_MAGIC);
    bytes.extend_from_slice(&dimensions.to_le_bytes());
    for field in [
        &definition.generation_id,
        &definition.space_id,
        &definition.encoder_id,
        &definition.revision,
        &definition.distance,
        &definition.normalization,
        &definition.input_role,
    ] {
        let length =
            u32::try_from(field.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(field.as_bytes());
    }
    Ok(bytes)
}

fn decode_definition(bytes: &[u8]) -> Result<SpaceDefinition, Error> {
    let mut cursor = 0;
    if take(bytes, &mut cursor, SPACE_MAGIC.len())? != SPACE_MAGIC {
        return Err(Error::new(ErrorCode::VectorIndexCorrupt));
    }
    let dimensions = usize::try_from(read_u32(bytes, &mut cursor)?)
        .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
    let generation_id = read_string(bytes, &mut cursor)?;
    let space_id = read_string(bytes, &mut cursor)?;
    let encoder_id = read_string(bytes, &mut cursor)?;
    let revision = read_string(bytes, &mut cursor)?;
    let distance = read_string(bytes, &mut cursor)?;
    let normalization = read_string(bytes, &mut cursor)?;
    let input_role = read_string(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(Error::new(ErrorCode::VectorIndexCorrupt));
    }
    let definition = SpaceDefinition {
        generation_id,
        space_id,
        encoder_id,
        revision,
        dimensions,
        distance,
        normalization,
        input_role,
    };
    validate_definition(&definition)?;
    Ok(definition)
}

fn read_pointer(path: &Path) -> Result<Option<String>, Error> {
    match fs::read(path) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(Error::new(ErrorCode::ReadFailed)),
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
    file.write_all(bytes)
        .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
    file.sync_data()
        .map_err(|_| Error::new(ErrorCode::SyncFailed))
}

fn write_replace(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let temporary = path.with_extension("next");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
    file.write_all(bytes)
        .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
    file.sync_data()
        .map_err(|_| Error::new(ErrorCode::SyncFailed))?;
    fs::rename(temporary, path).map_err(|_| Error::new(ErrorCode::WriteFailed))
}

fn sync_directory(directory: &Path) -> Result<(), Error> {
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|_| Error::new(ErrorCode::SyncFailed))
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, Error> {
    let length = usize::try_from(read_u32(bytes, cursor)?)
        .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
    String::from_utf8(take(bytes, cursor, length)?.to_vec())
        .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    take(bytes, cursor, 4).map(|value| u32::from_le_bytes(value.try_into().expect("four bytes")))
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, count: usize) -> Result<&'a [u8], Error> {
    let end = cursor
        .checked_add(count)
        .ok_or_else(|| Error::new(ErrorCode::VectorIndexCorrupt))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorCode::VectorIndexCorrupt))?;
    *cursor = end;
    Ok(value)
}
