initSidebarItems({"fn":[["deserialize_btreemap","Deserializes a BTreeMap from a list of tuples. Necessary when the keys are structs; see https://github.com/serde-rs/json/issues/402."],["deserialize_hashmap","Deserializes a HashMap from a list of tuples."],["deserialize_multimap","Deserializes a MultiMap."],["deserialize_usize","Deserializes a `usize` from a `u32`."],["from_binary","Deserializes an object from the bincode format."],["from_json","Deserializes an object from a JSON string."],["serialize_btreemap","Serializes a BTreeMap as a list of tuples. Necessary when the keys are structs; see https://github.com/serde-rs/json/issues/402."],["serialize_hashmap","Serializes a HashMap as a list of tuples, first sorting by the keys. This ensures the serialized form is deterministic."],["serialize_multimap","Serializes a MultiMap."],["serialize_usize","Serializes a `usize` as a `u32` to save space. Useful when you need `usize` for indexing, but the values don't exceed 2^32."],["serialized_size_bytes","The number of bytes for an object serialized to bincode."],["to_json","Stringifies an object to nicely formatted JSON."],["to_json_terse","Stringifies an object to terse JSON."]]});