use sha2::{Digest, Sha256};
use std::io::{Error, ErrorKind};
use transaction::{Amount, Input, Output, Transaction, Txid};
mod transaction;

const SEGWIT_MARKER: u8 = 0x00;
const SEGWIT_FLAG: u8 = 0x01;

// How many bytes of `raw` have already been consumed by `cursor`.
fn offset(raw: &[u8], cursor: &[u8]) -> usize {
    raw.len() - cursor.len()
}

fn read_bytes<'a>(transaction_bytes: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    if transaction_bytes.len() < n {
        return Err(Error::new(
            ErrorKind::UnexpectedEof,
            "unexpected end of transaction data",
        ));
    }
    let (head, tail) = transaction_bytes.split_at(n);
    *transaction_bytes = tail;
    Ok(head)
}

fn read_u8(transaction_bytes: &mut &[u8]) -> Result<u8, Error> {
    Ok(read_bytes(transaction_bytes, 1)?[0])
}

// Bitcoin uses little-endian encoding for most of its numeric fields, meaning
// the least significant byte comes first.
fn read_u32(bytes_slice: &mut &[u8]) -> Result<u32, Error> {
    let bytes = read_bytes(bytes_slice, 4)?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn read_u64(transaction_bytes: &mut &[u8]) -> Result<u64, Error> {
    let bytes = read_bytes(transaction_bytes, 8)?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

fn read_amount(transaction_bytes: &mut &[u8]) -> Result<Amount, Error> {
    let satoshis = read_u64(transaction_bytes)?;
    Ok(Amount::from_sat(satoshis))
}

// CompactSize (aka VarInt) is Bitcoin's variable-length integer encoding,
// used throughout the protocol for counts and script lengths:
//   0x00..=0xfc        -> value is the byte itself
//   0xfd + 2 bytes LE   -> u16 value
//   0xfe + 4 bytes LE   -> u32 value
//   0xff + 8 bytes LE   -> u64 value
fn read_compact_size(transaction_bytes: &mut &[u8]) -> Result<u64, Error> {
    match read_u8(transaction_bytes)? {
        0xfd => {
            let bytes = read_bytes(transaction_bytes, 2)?;
            Ok(u16::from_le_bytes(bytes.try_into().unwrap()) as u64)
        }
        0xfe => {
            let bytes = read_bytes(transaction_bytes, 4)?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()) as u64)
        }
        0xff => {
            let bytes = read_bytes(transaction_bytes, 8)?;
            Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
        }
        n => Ok(n as u64),
    }
}

fn read_txid(transaction_bytes: &mut &[u8]) -> Result<Txid, Error> {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(read_bytes(transaction_bytes, 32)?);
    Ok(Txid::from_bytes(bytes))
}

// A script is always prefixed by its length as a CompactSize.
fn read_script(transaction_bytes: &mut &[u8]) -> Result<Vec<u8>, Error> {
    let len = read_compact_size(transaction_bytes)? as usize;
    Ok(read_bytes(transaction_bytes, len)?.to_vec())
}

fn hash_raw_transaction(raw_transaction_bytes: &[u8]) -> Txid {
    // The TXID is the double-SHA256 of the legacy (non-witness) serialization.
    let first_hash = Sha256::digest(raw_transaction_bytes);
    let second_hash = Sha256::digest(first_hash);
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&second_hash);
    Txid::from_bytes(bytes)
}

pub fn decode_transaction(transaction_hex: String) -> Result<String, Box<dyn std::error::Error>> {
    let raw_bytes = hex::decode(transaction_hex.trim())?;
    let mut cursor: &[u8] = &raw_bytes;

    let version = read_u32(&mut cursor)?;

    // SegWit transactions insert a marker (0x00) and flag (0x01) byte right
    // after the version, which a legacy transaction would never have (the
    // marker byte would otherwise be read as a zero input count).
    let is_segwit = cursor.len() >= 2 && cursor[0] == SEGWIT_MARKER && cursor[1] == SEGWIT_FLAG;
    if is_segwit {
        read_u8(&mut cursor)?; // marker
        read_u8(&mut cursor)?; // flag
    }

    // The TXID is computed over version + inputs + outputs + locktime only,
    // so we track where that region starts and ends as we parse it.
    let legacy_start = offset(&raw_bytes, cursor);

    let input_count = read_compact_size(&mut cursor)?;
    let mut inputs = Vec::with_capacity(input_count as usize);
    for _ in 0..input_count {
        let txid = read_txid(&mut cursor)?;
        let output_index = read_u32(&mut cursor)?;
        let script_sig = read_script(&mut cursor)?;
        let sequence = read_u32(&mut cursor)?;
        inputs.push(Input {
            txid,
            output_index,
            script_sig,
            sequence,
            witness: Vec::new(),
        });
    }

    let output_count = read_compact_size(&mut cursor)?;
    let mut outputs = Vec::with_capacity(output_count as usize);
    for _ in 0..output_count {
        let amount = read_amount(&mut cursor)?;
        let script_pubkey = read_script(&mut cursor)?;
        outputs.push(Output {
            amount,
            script_pubkey,
        });
    }

    let legacy_end = offset(&raw_bytes, cursor);

    if is_segwit {
        // Every input carries its own witness stack, in input order.
        for input in inputs.iter_mut() {
            let item_count = read_compact_size(&mut cursor)?;
            let mut witness = Vec::with_capacity(item_count as usize);
            for _ in 0..item_count {
                let len = read_compact_size(&mut cursor)? as usize;
                witness.push(read_bytes(&mut cursor, len)?.to_vec());
            }
            input.witness = witness;
        }
    }

    let lock_time = read_u32(&mut cursor)?;

    if !cursor.is_empty() {
        return Err("unexpected trailing bytes after locktime".into());
    }

    let mut legacy_bytes = Vec::with_capacity((legacy_end - legacy_start) + 8);
    legacy_bytes.extend_from_slice(&raw_bytes[0..4]); // version
    legacy_bytes.extend_from_slice(&raw_bytes[legacy_start..legacy_end]); // inputs + outputs
    legacy_bytes.extend_from_slice(&lock_time.to_le_bytes());
    let transaction_id = hash_raw_transaction(&legacy_bytes);

    let transaction = Transaction {
        transaction_id,
        version,
        is_segwit,
        inputs,
        outputs,
        lock_time,
    };

    Ok(serde_json::to_string_pretty(&transaction)?)
}
