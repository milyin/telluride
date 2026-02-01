# CallbackEncode Trait System

## Overview

This implementation provides a flexible trait-based encoding system for `CallbackKey` that allows custom encoding/decoding implementations and performance optimizations.

The system provides two approaches:
1. **CallbackBitcode** - Opt-in trait with default bitcode encoding (recommended for most cases)
2. **CallbackEncode** - Direct implementation for completely custom encoding

## Architecture

### `CallbackEncode` Trait

The core trait for types that can be encoded/decoded to/from CallbackKey:

```rust
pub trait CallbackEncode: Hash + Clone + Send + Sync {
    fn encode_callback(&self) -> Vec<u8>;
    fn decode_callback(bytes: &[u8]) -> Result<Self, String>;
    fn bypass_encoding(&self) -> bool { false }
}
```

### `CallbackBitcode` Trait

A trait with default methods for bitcode-based encoding. Provides a convenient way to opt-in:

```rust
pub trait CallbackBitcode: bitcode::Encode + for<'a> bitcode::Decode<'a> + Hash + Clone + Send + Sync {
    fn encode_callback(&self) -> Vec<u8> {
        bitcode::encode(self)
    }
    
    fn decode_callback(bytes: &[u8]) -> Result<Self, String> {
        bitcode::decode(bytes).map_err(|e| e.to_string())
    }
    
    fn bypass_encoding(&self) -> bool {
        false
    }
}
```

Types that implement `CallbackBitcode` automatically get `CallbackEncode` via blanket implementation.

## Usage Patterns

### Pattern 1: Simple Bitcode Encoding (Recommended)

For most types, simply implement `CallbackBitcode` with an empty body:

```rust
#[derive(Debug, Clone, Hash, PartialEq, bitcode::Encode, bitcode::Decode)]
struct SimpleAction {
    action_type: u8,
    user_id: u64,
}

// Just implement CallbackBitcode - CallbackEncode is automatic
impl CallbackBitcode for SimpleAction {}

let key = CallbackKey::pack(&simple_action, &storage).await;
```

### Pattern 2: Bitcode with Custom Bypass Logic

Override `bypass_encoding()` to control when encoding is skipped:

```rust
#[derive(Debug, Clone, Hash, PartialEq, bitcode::Encode, bitcode::Decode)]
struct LargeAction {
    data: Vec<u8>,
}

impl CallbackBitcode for LargeAction {
    fn bypass_encoding(&self) -> bool {
        // Skip inline encoding for large data
        self.data.len() > 50
    }
}
```

### Pattern 3: Custom Bitcode Encoding

Override encoding methods while still using bitcode:

```rust
#[derive(Debug, Clone, Hash, PartialEq, bitcode::Encode, bitcode::Decode)]
struct CustomBitcodeType {
    value: u32,
}

impl CallbackBitcode for CustomBitcodeType {
    fn encode_callback(&self) -> Vec<u8> {
        // Add magic bytes prefix
        let mut encoded = vec![0xCA, 0xFE];
        encoded.extend_from_slice(&bitcode::encode(self));
        encoded
    }
    
    fn decode_callback(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 2 || &bytes[0..2] != [0xCA, 0xFE] {
            return Err("Missing magic prefix".to_string());
        }
        bitcode::decode(&bytes[2..]).map_err(|e| e.to_string())
    }
}
```

### Pattern 4: Fully Custom Encoding

For types that don't use bitcode at all, implement `CallbackEncode` directly:

```rust
#[derive(Debug, Clone, PartialEq)]
struct CustomAction {
    data: String,
}

impl Hash for CustomAction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl CallbackEncode for CustomAction {
    fn encode_callback(&self) -> Vec<u8> {
        self.data.as_bytes().to_vec()
    }
    
    fn decode_callback(bytes: &[u8]) -> Result<Self, String> {
        String::from_utf8(bytes.to_vec())
            .map(|data| CustomAction { data })
            .map_err(|e| e.to_string())
    }
}
```

## How It Works

1. **Packing**: When `CallbackKey::pack()` is called:
   - First checks if `bypass_encoding()` returns true
   - If yes, skips inline encoding and stores directly
   - Otherwise, encodes using `encode_callback()`
   - If encoded data fits in 64 bytes (with prefix), it's embedded inline
   - If too large, it's stored and referenced by hash

2. **Unpacking**: When `CallbackKey::unpack()` is called:
   - Checks the prefix to determine if data is inline or storage-backed
   - For inline data, decodes using `decode_callback()`
   - For storage data, looks up the value from storage

## Benefits

1. **Explicit Opt-in**: Write `impl CallbackBitcode for MyType {}` to get bitcode encoding
2. **Customization**: Override default methods for custom behavior while keeping bitcode
3. **Full Control**: Implement `CallbackEncode` directly for completely custom encoding
4. **Performance**: `bypass_encoding()` allows skipping encoding for known-large instances
5. **Type Safety**: Trait bounds ensure only compatible types can be packed/unpacked

## Decision Tree

```
Do you need completely custom encoding (not bitcode)?
├─ Yes → Implement CallbackEncode directly
└─ No → Implement CallbackBitcode
    ├─ Need standard bitcode? → impl CallbackBitcode for T {}
    ├─ Need custom bypass logic? → Override bypass_encoding()
    └─ Need custom encoding with bitcode? → Override encode/decode methods
```

## Example

See [examples/custom_callback_encoding.rs](../examples/custom_callback_encoding.rs) for a complete working example demonstrating all usage patterns.

## Migration Guide

**From previous version**: Types now need explicit implementation.

### Before (no implementation needed):
```rust
#[derive(Debug, Clone, Hash, bitcode::Encode, bitcode::Decode)]
struct MyAction {
    value: u32,
}
// CallbackEncode was automatically implemented via blanket impl
```

### After (explicit opt-in required):
```rust
#[derive(Debug, Clone, Hash, bitcode::Encode, bitcode::Decode)]
struct MyAction {
    value: u32,
}

// Just add this one line:
impl CallbackBitcode for MyAction {}
```

This explicit opt-in approach provides:
- **Clarity**: It's clear which types support CallbackKey encoding
- **Control**: You can easily customize behavior by overriding methods
- **No Magic**: No automatic blanket implementations based on traits

## API Summary

- **Trait**: `CallbackBitcode` - Opt-in trait with default bitcode methods (recommended)
- **Trait**: `CallbackEncode` - Core trait for all encoding (automatically implemented for `CallbackBitcode`)
- **Required**: `CallbackKey::pack()` requires `V: CallbackEncode`
- **Required**: `CallbackKey::unpack()` requires `V: CallbackEncode`
