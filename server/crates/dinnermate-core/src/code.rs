use rand::Rng;

/// No I/L/O/0/1 — codes are read aloud and typed, so drop ambiguous glyphs.
pub const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
pub const CODE_LEN: usize = 6;

pub fn generate_code(rng: &mut impl Rng) -> String {
    (0..CODE_LEN)
        .map(|_| CODE_ALPHABET[rng.random_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn code_has_length_6() {
        let code = generate_code(&mut StdRng::seed_from_u64(1));
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn code_uses_only_alphabet_chars() {
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..100 {
            let code = generate_code(&mut rng);
            assert!(
                code.bytes().all(|b| CODE_ALPHABET.contains(&b)),
                "code {code:?} contains chars outside the alphabet"
            );
        }
    }

    #[test]
    fn different_seeds_produce_different_codes() {
        let a = generate_code(&mut StdRng::seed_from_u64(1));
        let b = generate_code(&mut StdRng::seed_from_u64(2));
        assert_ne!(a, b);
    }

    #[test]
    fn same_seed_produces_same_code() {
        let a = generate_code(&mut StdRng::seed_from_u64(42));
        let b = generate_code(&mut StdRng::seed_from_u64(42));
        assert_eq!(a, b);
    }
}
