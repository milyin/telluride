/// Example demonstrating custom callback encoding with the CallbackEncode trait
/// and the bypass_encoding feature for large instances.
use std::hash::Hash;
use std::sync::Arc;
use telluride::command::{CallbackEncode, CallbackKey};
use telluride::data_store::{InMemStore, UserProxy};
use teloxide::types::UserId;

// Example 1: Using bitcode implementation (standard pattern)
#[derive(Debug, Clone, Hash, PartialEq, bitcode::Encode, bitcode::Decode)]
struct SimpleAction {
    action_type: u8,
    user_id: u64,
}

// Just implement CallbackBitcode with empty body to get bitcode encoding
impl telluride::command::CallbackBitcode for SimpleAction {}

// Example 2: Custom encoding implementation (no bitcode traits)
#[derive(Debug, Clone, PartialEq)]
struct CustomEncodedAction {
    data: String,
}

impl Hash for CustomEncodedAction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

// Manual implementation of CallbackEncode for types that DON'T use bitcode
impl CallbackEncode for CustomEncodedAction {
    fn encode_callback(&self) -> Vec<u8> {
        // Custom encoding: just use the string bytes
        self.data.as_bytes().to_vec()
    }

    fn decode_callback(bytes: &[u8]) -> Result<Self, telluride::command::UnpackError> {
        // Custom decoding: convert bytes back to string
        String::from_utf8(bytes.to_vec())
            .map(|data| CustomEncodedAction { data })
            .map_err(|e| telluride::command::UnpackError::DeserializeError(e.to_string()))
    }
}

// Example 3: Type that uses bitcode internally but wants custom bypass logic
// Since the wrapper doesn't implement bitcode traits, implement CallbackEncode directly
#[derive(Debug, Clone, Hash, PartialEq, bitcode::Encode, bitcode::Decode)]
struct LargeData {
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Hash, PartialEq)]
struct LargeAction(LargeData);

// Implement CallbackEncode directly (not via CallbackBitcode)
impl telluride::command::CallbackEncode for LargeAction {
    fn encode_callback(&self) -> Vec<u8> {
        bitcode::encode(&self.0)
    }

    fn decode_callback(bytes: &[u8]) -> Result<Self, telluride::command::UnpackError> {
        bitcode::decode::<LargeData>(bytes)
            .map(LargeAction)
            .map_err(|e| telluride::command::UnpackError::DeserializeError(e.to_string()))
    }

    fn bypass_encoding(&self) -> bool {
        // If payload is larger than 50 bytes, skip inline encoding
        // and go straight to storage
        self.0.payload.len() > 50
    }
}

#[tokio::main]
async fn main() {
    let user_id = UserId(12345);

    // Example 1: Default bitcode encoding (small instance)
    let storage1 = Arc::new(InMemStore::new());
    let user_store1 = UserProxy::new(storage1, user_id);

    let simple_action = SimpleAction {
        action_type: 1,
        user_id: 12345,
    };
    let key1 = CallbackKey::pack(simple_action.clone(), &user_store1).await;
    println!("Simple action key: {}", key1);
    println!("Is inline: {}", key1.is_inline());

    let unpacked1 = CallbackKey::unpack::<SimpleAction, _>(key1.as_str(), &user_store1)
        .await
        .expect("Should unpack");
    assert_eq!(unpacked1, simple_action);
    println!("✓ Simple action unpacked successfully\n");

    // Example 2: Custom encoding
    let storage2 = Arc::new(InMemStore::new());
    let user_store2 = UserProxy::new(storage2, user_id);

    let custom_action = CustomEncodedAction {
        data: "custom_data".to_string(),
    };
    let key2 = CallbackKey::pack(custom_action.clone(), &user_store2).await;
    println!("Custom encoded action key: {}", key2);
    println!("Is inline: {}", key2.is_inline());

    let unpacked2 = CallbackKey::unpack::<CustomEncodedAction, _>(key2.as_str(), &user_store2)
        .await
        .expect("Should unpack");
    assert_eq!(unpacked2.data, custom_action.data);
    println!("✓ Custom encoded action unpacked successfully\n");

    // Example 3: Small instance (normal encoding)
    let storage3 = Arc::new(InMemStore::new());
    let user_store3 = UserProxy::new(storage3, user_id);

    let small_action = LargeAction(LargeData {
        payload: vec![1, 2, 3, 4, 5], // Small data
    });
    let key3 = CallbackKey::pack(small_action.clone(), &user_store3).await;
    println!("Small action key: {}", key3);
    println!("Is inline: {} (should be true)", key3.is_inline());

    let unpacked3 = CallbackKey::unpack::<LargeAction, _>(key3.as_str(), &user_store3)
        .await
        .expect("Should unpack");
    assert_eq!(unpacked3.0, small_action.0);
    println!("✓ Small action unpacked successfully\n");

    // Example 4: Large instance (bypasses encoding)
    let storage4 = Arc::new(InMemStore::new());
    let user_store4 = UserProxy::new(storage4, user_id);

    let large_action = LargeAction(LargeData {
        payload: vec![0; 100], // Large data (> 50 bytes)
    });
    let key4 = CallbackKey::pack(large_action.clone(), &user_store4).await;
    println!("Large action key: {}", key4);
    println!(
        "Is storage-backed: {} (should be true due to bypass)",
        key4.is_storage_backed()
    );

    let unpacked4 = CallbackKey::unpack::<LargeAction, _>(key4.as_str(), &user_store4)
        .await
        .expect("Should unpack");
    assert_eq!(unpacked4.0.payload, large_action.0.payload);
    println!("✓ Large action unpacked successfully (was stored, not inlined)\n");

    println!("All examples completed successfully!");
}
