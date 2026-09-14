use super::*;
use poe_optimizer_core::owned_schema::DefinitionSchemaIndex;
use std::io;

pub fn decode_registry(bytes: &[u8], limits: OwnedMappingLimits) -> Result<OwnedIdRegistry> {
    check_bytes(bytes, limits)?;
    OwnedIdRegistry::new(serde_json::from_slice(bytes)?, limits)
}
pub fn encode_registry(registry: &OwnedIdRegistry, limits: OwnedMappingLimits) -> Result<Vec<u8>> {
    registry.validate_limits(limits)?;
    bounded_json(registry.input(), limits.max_wire_bytes)
}
pub fn decode_mapping_package<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    registry: &OwnedIdRegistry,
    definitions: &I,
    limits: OwnedMappingLimits,
) -> Result<OwnedMappingIndex> {
    check_bytes(bytes, limits)?;
    OwnedMappingIndex::new(
        serde_json::from_slice(bytes)?,
        registry,
        definitions,
        limits,
    )
}
pub fn encode_mapping_package(
    mapping: &OwnedMappingIndex,
    limits: OwnedMappingLimits,
) -> Result<Vec<u8>> {
    mapping.validate_limits(limits)?;
    Ok(mapping.bytes().to_vec())
}
fn check_bytes(bytes: &[u8], limits: OwnedMappingLimits) -> Result<()> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(OwnedMappingError::TooLarge {
            maximum: limits.max_wire_bytes,
        });
    }
    Ok(())
}

pub(super) fn bounded_json(value: &impl Serialize, maximum: usize) -> Result<Vec<u8>> {
    struct Writer {
        bytes: Vec<u8>,
        maximum: usize,
        exceeded: bool,
    }
    impl io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > self.maximum - self.bytes.len() {
                self.exceeded = true;
                return Err(io::Error::other("owned mapping byte limit exceeded"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer {
        bytes: vec![],
        maximum,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded {
        return Err(OwnedMappingError::TooLarge { maximum });
    }
    result?;
    Ok(writer.bytes)
}
pub(super) fn serialized_len(value: &impl Serialize, maximum: usize) -> Result<usize> {
    struct Counter {
        written: usize,
        maximum: usize,
        exceeded: bool,
    }
    impl io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > self.maximum - self.written {
                self.exceeded = true;
                return Err(io::Error::other("owned mapping byte limit exceeded"));
            }
            self.written += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter {
        written: 0,
        maximum,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut counter, value);
    if counter.exceeded {
        return Err(OwnedMappingError::TooLarge { maximum });
    }
    result?;
    Ok(counter.written)
}
