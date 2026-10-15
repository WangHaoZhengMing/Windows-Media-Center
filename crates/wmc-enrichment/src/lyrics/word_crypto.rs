//! Anonymous NetEase EAPI request encoding and QQ's nonstandard QRC transport.
// QRC implementation adapted from apoint123/qrc-decoder (MIT).
// Copyright (c) 2025 apoint123. See qrc-decoder.LICENSE.
use super::qrc_tables::*;
use aes::{
    Aes128,
    cipher::{BlockEncrypt, KeyInit},
};
use anyhow::{Context, Result, ensure};
use md5::{Digest, Md5};
use std::io::Read;

pub(super) fn eapi(path: &str, value: &serde_json::Value) -> String {
    let json = value.to_string();
    let hash = Md5::digest(format!("nobody{path}use{json}md5forencrypt"));
    let mut bytes = format!("{path}-36cd479b6b5-{json}-36cd479b6b5-{hash:x}").into_bytes();
    let padding = 16 - bytes.len() % 16;
    bytes.resize(bytes.len() + padding, padding as u8);
    let cipher = Aes128::new_from_slice(b"e82ckenh8dichen8").expect("AES key length");
    for block in bytes.as_chunks_mut::<16>().0 {
        cipher.encrypt_block(block.into());
    }
    format!(
        "params={}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<String>()
    )
}

fn permute(input: u64, bits: u32, table: &[u8]) -> u64 {
    table.iter().fold(0, |value, bit| {
        (value << 1) | ((input >> (bits - u32::from(*bit))) & 1)
    })
}
fn schedule(key: &[u8; 8], decrypt: bool) -> [u64; 16] {
    let mut key = *key;
    key[..4].reverse();
    key[4..].reverse();
    let key = u64::from_be_bytes(key);
    let select = |table: &[u8]| {
        table
            .iter()
            .fold(0u32, |v, bit| (v << 1) | ((key >> (63 - bit)) & 1) as u32)
    };
    let (mut c, mut d) = (select(&KEY_PERM_C), select(&KEY_PERM_D));
    let mut keys = [0; 16];
    for (round, shift) in KEY_RND_SHIFT.iter().enumerate() {
        c = ((c << shift) | (c >> (28 - shift))) & 0x0fff_ffff;
        d = ((d << shift) | (d >> (28 - shift))) & 0x0fff_ffff;
        let subkey = KEY_COMPRESSION.iter().fold(0u64, |v, pos| {
            // QQ's D-half selection is deliberately offset by one bit.
            let bit = if *pos < 28 {
                (c >> (27 - pos)) & 1
            } else {
                ((d << 4) >> (58 - pos)) & 1
            };
            (v << 1) | u64::from(bit)
        });
        keys[if decrypt { 15 - round } else { round }] = subkey;
    }
    keys
}
fn crypt(block: u64, keys: &[u64; 16]) -> u64 {
    let state = permute(block, 64, &IP_RULE);
    let (mut left, mut right) = ((state >> 32) as u32, state as u32);
    for key in keys {
        let expanded = permute(u64::from(right), 32, &E_BOX_TABLE) ^ key;
        let mut substituted = 0u32;
        for (i, sbox) in S_BOXES.iter().enumerate() {
            let a = ((expanded >> (42 - i * 6)) & 63) as usize;
            let index = (a & 32) | ((a & 31) >> 1) | ((a & 1) << 4);
            substituted = (substituted << 4) | u32::from(sbox[index]);
        }
        (left, right) = (
            right,
            left ^ permute(u64::from(substituted), 32, &P_BOX) as u32,
        );
    }
    permute((u64::from(right) << 32) | u64::from(left), 64, &INV_IP_RULE)
}
pub(super) fn decrypt_qrc(hex: &str) -> Result<String> {
    let hex = hex.trim();
    ensure!(
        !hex.is_empty()
            && hex.len().is_multiple_of(16)
            && hex.len() <= 8 * 1024 * 1024
            && hex.bytes().all(|c| c.is_ascii_hexdigit()),
        "无效的 QRC 密文"
    );
    let keys = [
        schedule(b"!@#)(NHL", true),
        schedule(b"123ZXC!@", false),
        schedule(b"!@#)(*$%", true),
    ];
    let mut decrypted = Vec::with_capacity(hex.len() / 2);
    for block in hex.as_bytes().as_chunks::<16>().0 {
        let mut word = u64::from_str_radix(std::str::from_utf8(block)?, 16)?;
        for key in &keys {
            word = crypt(word, key);
        }
        decrypted.extend_from_slice(&word.to_be_bytes());
    }
    let mut plain = Vec::new();
    flate2::read::ZlibDecoder::new(decrypted.as_slice())
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut plain)
        .context("QRC 解压失败")?;
    ensure!(plain.len() <= 4 * 1024 * 1024, "QRC 解压结果过大");
    Ok(String::from_utf8(plain)?
        .trim_start_matches('\u{feff}')
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_qrc_vector_decrypts_and_parses() {
        // MIT qrc-decoder interoperability vector; see qrc-decoder.LICENSE.
        let text = decrypt_qrc(include_str!("testdata/qrc.hex")).unwrap();
        let lyrics = wmc_lyrics::Lyrics::parse_text(&text, 3600.).unwrap();
        assert!(!lyrics.lines.is_empty());
        assert!(
            lyrics
                .lines
                .iter()
                .any(|line| line.word_synced && line.words.len() > 1)
        );
    }
    #[test]
    fn malformed_ciphertext_is_rejected() {
        for text in ["", "00", "zzzzzzzzzzzzzzzz", "0000000000000000"] {
            assert!(decrypt_qrc(text).is_err());
        }
    }
}
