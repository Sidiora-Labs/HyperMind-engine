import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { Client } from "./client";
import { UsageClient, decodeUsageView, decodeProviderUsage, UsageDecodeError } from "./index";

test("actual authenticated daemon usage and independently recorded parser vectors",async()=>{
  const path=process.env.HM_USAGE_CLIENT_CONFIG;
  if(!path)throw new Error("actual native usage fixture and daemon required");
  const config=JSON.parse(fs.readFileSync(path,"utf8"));
  const client=await Client.connect({capabilityToken:Buffer.from(config.token_hex,"hex"),grpc:{target:config.target,ca:fs.readFileSync(config.ca),certificate:fs.readFileSync(config.certificate),privateKey:fs.readFileSync(config.private_key)}});
  try {
    const usage=await new UsageClient(client,config.scope,7).inspect();
    const native=JSON.parse(fs.readFileSync(config.expected,"utf8"));
    assert.deepEqual(usage,decodeUsageView(native,config.scope));
    assert.equal(usage.rollup.held_tokens,9007199254740993n);
    assert.equal(usage.rollup.known_tokens,0n);
    assert.equal(usage.rollup.unknown_reservations,1n);
    assert.equal(usage.rollup.observed_calls,0n);
    assert.equal(usage.state.limits.total_tokens,18446744073709551615n);
    assert.equal(usage.state.reservations[0].settled_tokens,null);
    assert.equal(usage.state.reservations[0].attribution.source_ids[0],"measurement-record");
    assert.equal(usage.state.observations.length,0);
    assert.equal(usage.rollup.attributions[0].turn_id,"precision-turn");
    const recorded=JSON.parse(fs.readFileSync(config.vectors,"utf8"));
    assert.equal(recorded.origin,"recorded_parser_vectors_only");assert.equal(recorded.billed,false);
    const tokens=decodeProviderUsage(recorded.tokens),quota=decodeProviderUsage(recorded.quota);
    assert.equal(tokens.tokens.input,9007199254740993n);assert.equal(tokens.tokens.output,0n);assert.equal(tokens.tokens.cache_read,null);
    assert.equal(tokens.reported_charge?.magnitude.nanodollar_numerator,9007199254740993n);
    assert.equal(tokens.expires_at_ns,null);
    assert.deepEqual(quota.quota_windows[0].limit,{kind:"count",value:18446744073709551615n});
    assert.deepEqual(quota.quota_windows[0].remaining,{kind:"count",value:0n});assert.equal(quota.quota_windows[0].used,null);
    assert.equal(quota.quota_windows[0].starts_at_ns,-9007199254740993n);assert.equal(quota.quota_windows[0].resets_at_ns,9223372036854775807n);
    assert.equal(quota.quota_windows[0].interval_ms,9007199254740993n);
    const absent=structuredClone(recorded.tokens);delete absent.tokens.cache_read;
    assert.equal(Object.hasOwn(decodeProviderUsage(absent).tokens,"cache_read"),false);
    const legacy=structuredClone(native);legacy.rollup.known_tokens=0;assert.equal(decodeUsageView(legacy).rollup.known_tokens,0n);
    for(const malformed of [9007199254740993,"01","+1","-0","18446744073709551616","1.0","1\n","1 ",true]){
      const bad=structuredClone(native);bad.rollup.held_tokens=malformed;assert.throws(()=>decodeUsageView(bad),UsageDecodeError);
    }
    for(const malformed of [9007199254740993,"9223372036854775808","-9223372036854775809","+1","-0"]){const bad=structuredClone(recorded.tokens);bad.observed_at_ns=malformed;assert.throws(()=>decodeProviderUsage(bad),UsageDecodeError);}
    const leaked=structuredClone(recorded.tokens);leaked.raw={content:"excluded"};assert.throws(()=>decodeProviderUsage(leaked),UsageDecodeError);
  } finally {await client.close();}
});
