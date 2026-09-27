//! Factorial ranking of permutations, and base-`b` orientation codes.
//!
//! Ranking matches Kociemba's `get_corners`: rotate the prefix `0..=j` left
//! until position `j` is fixed, and accumulate those rotation counts in
//! base `(j+1)`. Orientation codes use the first `n-1` entries as digits
//! (most significant first). Decoding restores the last entry so the sum of
//! all entries is `0` mod `base`.

fn rotate_left(a: &mut [u8], left: usize, right: usize) {
    let first = a[left];
    let mut i = left;
    while i < right {
        a[i] = a[i + 1];
        i += 1;
    }
    a[right] = first;
}

fn rotate_right(a: &mut [u8], left: usize, right: usize) {
    let last = a[right];
    let mut i = right;
    while i > left {
        a[i] = a[i - 1];
        i -= 1;
    }
    a[left] = last;
}

/// Rank of a permutation of `0..n` written in place. `n` is `perm.len()`, 1..=12.
pub fn rank_permutation(perm: &[u8]) -> u32 {
    let n = perm.len();
    assert!(
        (1..=12).contains(&n),
        "permutation length {n} is out of range"
    );
    let mut p = [0u8; 12];
    p[..n].copy_from_slice(perm);
    let mut rank = 0u32;
    let mut j = n - 1;
    while j >= 1 {
        let mut k = 0u32;
        while p[j] != j as u8 {
            rotate_left(&mut p[..n], 0, j);
            k += 1;
            assert!(k <= j as u32, "not a permutation");
        }
        rank = (j as u32 + 1) * rank + k;
        j -= 1;
    }
    rank
}

/// Write the permutation of `0..n` with this rank into `out` (`out.len() == n`).
pub fn unrank_permutation(n: usize, mut rank: u32, out: &mut [u8]) {
    assert_eq!(out.len(), n);
    assert!(
        (1..=12).contains(&n),
        "permutation length {n} is out of range"
    );
    for i in 0..n {
        out[i] = i as u8;
    }
    for j in 0..n {
        let mut k = rank % (j as u32 + 1);
        rank /= j as u32 + 1;
        while k > 0 {
            rotate_right(out, 0, j);
            k -= 1;
        }
    }
}

/// Base-`base` code of `ori[..len-1]`. The last entry is ignored.
pub fn encode_orientation(ori: &[u8], base: u8) -> u32 {
    assert!(ori.len() >= 2, "orientation needs a dependent last cubie");
    assert!(base >= 2, "orientation base must be at least 2");
    let mut code = 0u32;
    for &digit in &ori[..ori.len() - 1] {
        assert!(
            digit < base,
            "orientation digit {digit} is not in base {base}"
        );
        code = code * u32::from(base) + u32::from(digit);
    }
    code
}

/// Inverse of [`encode_orientation`]. The last entry is `(base - sum) mod base`.
pub fn decode_orientation(mut rank: u32, base: u8, out: &mut [u8]) {
    assert!(out.len() >= 2, "orientation needs a dependent last cubie");
    assert!(base >= 2, "orientation base must be at least 2");
    let n = out.len();
    let mut sum = 0u8;
    for i in (0..n - 1).rev() {
        let digit = (rank % u32::from(base)) as u8;
        out[i] = digit;
        sum = sum.wrapping_add(digit);
        rank /= u32::from(base);
    }
    out[n - 1] = (base - sum % base) % base;
}

/// Binomial coefficient C(n, k). Matches Kociemba's `c_nk` for n <= 12.
pub fn binomial(n: usize, k: usize) -> u32 {
    if k > n {
        return 0;
    }
    let mut k = k;
    if k > n - k {
        k = n - k;
    }
    let mut s = 1u32;
    let mut i = n;
    let mut j = 1usize;
    while i != n - k {
        s *= i as u32;
        s /= j as u32;
        i -= 1;
        j += 1;
    }
    s
}

/// Inversion parity of `perm`. 0 is even.
pub fn permutation_parity(perm: &[u8]) -> u8 {
    let mut inversions = 0u32;
    for i in (1..perm.len()).rev() {
        for j in (0..i).rev() {
            if perm[j] > perm[i] {
                inversions += 1;
            }
        }
    }
    (inversions & 1) as u8
}

/// UD-slice coordinate, 0..494. Positions of edges FR, FL, BL, BR, order ignored.
pub fn ud_slice(ep: &[u8; 12]) -> u16 {
    let mut a = 0u32;
    let mut x = 0usize;
    for j in (0..12).rev() {
        if (8..=11).contains(&ep[j]) {
            a += binomial(11 - j, x + 1);
            x += 1;
        }
    }
    a as u16
}

/// Sorted UD-slice coordinate, 0..11879. In phase 2 this is the 4! slice permutation.
pub fn ud_slice_sorted(ep: &[u8; 12]) -> u16 {
    let mut a = 0u32;
    let mut x = 0usize;
    let mut edge4 = [0u8; 4];
    for j in (0..12).rev() {
        if (8..=11).contains(&ep[j]) {
            a += binomial(11 - j, x + 1);
            edge4[3 - x] = ep[j];
            x += 1;
        }
    }
    debug_assert_eq!(x, 4, "a legal edge permutation has four slice edges");
    let mut b = 0u32;
    for j in (1..4).rev() {
        let mut k = 0u32;
        while edge4[j] != j as u8 + 8 {
            rotate_left(&mut edge4, 0, j);
            k += 1;
            assert!(k <= 4, "slice edges are not FR FL BL BR");
        }
        b = (j as u32 + 1) * b + k;
    }
    (24 * a + b) as u16
}

/// Phase-2 coordinate of the eight U/D edges. They must occupy the eight U/D slots.
pub fn ud_edges(ep: &[u8; 12]) -> u16 {
    rank_permutation(&ep[..8]) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_permutation_is_zero() {
        assert_eq!(rank_permutation(&[0, 1, 2, 3, 4, 5, 6, 7]), 0);
        assert_eq!(rank_permutation(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]), 0);
    }

    #[test]
    fn corner_permutations_roundtrip() {
        let mut perm = [0u8; 8];
        for rank in 0..40_320u32 {
            unrank_permutation(8, rank, &mut perm);
            assert_eq!(rank_permutation(&perm), rank);
        }
    }

    #[test]
    fn edge_permutation_samples_roundtrip() {
        let mut perm = [0u8; 12];
        for rank in [0u32, 1, 24, 1_234_567, 479_001_599] {
            unrank_permutation(12, rank, &mut perm);
            assert_eq!(rank_permutation(&perm), rank);
        }
    }

    #[test]
    fn kociemba_twist_example_is_1494() {
        // 2·3^6 + 1·3^3 + 1·3^2 = 1494. Last twist makes the total 0 mod 3.
        let ori = [2, 0, 0, 1, 1, 0, 0, 2];
        assert_eq!(encode_orientation(&ori, 3), 1494);
        let mut out = [0u8; 8];
        decode_orientation(1494, 3, &mut out);
        assert_eq!(out, ori);
    }

    #[test]
    fn twist_and_flip_roundtrip() {
        let mut twist = [0u8; 8];
        for rank in 0..2187u32 {
            decode_orientation(rank, 3, &mut twist);
            assert_eq!(encode_orientation(&twist, 3), rank);
            assert_eq!(twist.iter().map(|d| u32::from(*d)).sum::<u32>() % 3, 0);
        }
        let mut flip = [0u8; 12];
        for rank in 0..2048u32 {
            decode_orientation(rank, 2, &mut flip);
            assert_eq!(encode_orientation(&flip, 2), rank);
            assert_eq!(flip.iter().map(|d| u32::from(*d)).sum::<u32>() % 2, 0);
        }
    }

    #[test]
    fn binomial_and_solved_slice_are_zero() {
        assert_eq!(binomial(12, 4), 495);
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(0, 1), 0);
        let solved = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        assert_eq!(ud_slice(&solved), 0);
        assert_eq!(ud_slice_sorted(&solved), 0);
        assert_eq!(permutation_parity(&[0, 1, 2, 3]), 0);
        assert_eq!(permutation_parity(&[1, 0, 2, 3]), 1);
    }
}
