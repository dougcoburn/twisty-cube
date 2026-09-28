//! Move tables and pruning tables.
//!
//! Files in `tables/`, little-endian, checked by `manifest.bin` (magic `CUBT`,
//! format version, then one IEEE CRC-32 per file):
//!
//! * `twist_move.bin` 78,732 bytes — twist × 18 moves, `u16`
//! * `flip_move.bin` 73,728 — flip × 18, `u16`
//! * `slice_move.bin` 17,820 — UD-slice combination × 18, `u16`
//! * `slice_sorted_move.bin` 427,680 — sorted slice × 18, `u16`
//! * `corners_move.bin` 1,451,520 — corner perm × 18, `u16`
//! * `ud_edges_move.bin` 1,451,520 — UD-edge perm × 18, `u16` (`u16::MAX` outside H)
//! * `prune_twist_slice.bin` 1,082,565 — two-phase lower bound, `u8`
//! * `prune_flip_slice.bin` 1,013,760 — two-phase lower bound, `u8`
//! * `prune_corners.bin` 967,680 — phase-2 corners × slice perm, `u8`
//! * `prune_ud_edges.bin` 967,680 — phase-2 UD-edges × slice perm, `u8`
//!
//! `manifest.bin` covers those ten files. `phase1_prun.bin` (140,908,410 bytes,
//! exact moves to H_UD) and `phase1_manifest.bin` are built only by
//! `load_optimal`. Two-phase search does not read them.
//!
//! The split two-phase prunes are lower bounds. `phase1_prun.bin` is the exact
//! distance to H_UD, indexed by flip-slice symmetry class (64430) then twist (2187).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use memmap2::Mmap;

use crate::moves::{phase2_move, Move, ALL_MOVES, MOVE_COUNT, PHASE2_MOVES};
use crate::sym::{sym_index, N_PHASE1, N_SYM};
use crate::CubieCube;

pub const N_TWIST: usize = 2187;
pub const N_FLIP: usize = 2048;
pub const N_SLICE: usize = 495;
pub const N_SLICE_SORTED: usize = 11_880;
pub const N_CORNERS: usize = 40_320;
pub const N_UD_EDGES: usize = 40_320;
pub const N_SLICE_PERM: usize = 24;
pub const TABLE_BUDGET: u64 = 200 * 1024 * 1024;

/// Move-table entry for a UD-edge coordinate after a move that leaves H.
pub const UD_EDGES_INVALID: u16 = u16::MAX;

const UNSEEN: u8 = 255;
/// Two-phase manifest. Version 3 dropped `phase1_prun.bin` from this file.
const FORMAT_VERSION: u32 = 3;
const PHASE1_VERSION: u32 = 1;
const MANIFEST: &str = "manifest.bin";
const PHASE1_FILE: &str = "phase1_prun.bin";
const PHASE1_MANIFEST: &str = "phase1_manifest.bin";
const MANIFEST_MAGIC: &[u8; 4] = b"CUBT";

struct Spec {
    name: &'static str,
    bytes: usize,
}

fn specs() -> [Spec; 10] {
    [
        Spec {
            name: "twist_move.bin",
            bytes: N_TWIST * MOVE_COUNT * 2,
        },
        Spec {
            name: "flip_move.bin",
            bytes: N_FLIP * MOVE_COUNT * 2,
        },
        Spec {
            name: "slice_move.bin",
            bytes: N_SLICE * MOVE_COUNT * 2,
        },
        Spec {
            name: "slice_sorted_move.bin",
            bytes: N_SLICE_SORTED * MOVE_COUNT * 2,
        },
        Spec {
            name: "corners_move.bin",
            bytes: N_CORNERS * MOVE_COUNT * 2,
        },
        Spec {
            name: "ud_edges_move.bin",
            bytes: N_UD_EDGES * MOVE_COUNT * 2,
        },
        Spec {
            name: "prune_twist_slice.bin",
            bytes: N_TWIST * N_SLICE,
        },
        Spec {
            name: "prune_flip_slice.bin",
            bytes: N_FLIP * N_SLICE,
        },
        Spec {
            name: "prune_corners.bin",
            bytes: N_CORNERS * N_SLICE_PERM,
        },
        Spec {
            name: "prune_ud_edges.bin",
            bytes: N_UD_EDGES * N_SLICE_PERM,
        },
    ]
}

pub struct LoadedTables {
    pub tables: Tables,
    /// `true` when every file was already on disk and this run only mapped it.
    pub mapped: bool,
    pub files: Vec<(String, u64)>,
}

pub struct Tables {
    twist_move: Mmap,
    flip_move: Mmap,
    slice_move: Mmap,
    slice_sorted_move: Mmap,
    corners_move: Mmap,
    ud_edges_move: Mmap,
    prune_twist_slice: Mmap,
    prune_flip_slice: Mmap,
    prune_corners: Mmap,
    prune_ud_edges: Mmap,
    phase1_prun: Option<Mmap>,
}

impl LoadedTables {
    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|(_, n)| *n).sum()
    }
}

impl Tables {
    pub fn twist_move(&self, twist: u16, mv: Move) -> u16 {
        read_u16(&self.twist_move, twist as usize * MOVE_COUNT + mv as usize)
    }

    pub fn flip_move(&self, flip: u16, mv: Move) -> u16 {
        read_u16(&self.flip_move, flip as usize * MOVE_COUNT + mv as usize)
    }

    pub fn slice_move(&self, slice: u16, mv: Move) -> u16 {
        read_u16(&self.slice_move, slice as usize * MOVE_COUNT + mv as usize)
    }

    pub fn slice_sorted_move(&self, slice: u16, mv: Move) -> u16 {
        read_u16(
            &self.slice_sorted_move,
            slice as usize * MOVE_COUNT + mv as usize,
        )
    }

    pub fn corners_move(&self, corners: u16, mv: Move) -> u16 {
        read_u16(
            &self.corners_move,
            corners as usize * MOVE_COUNT + mv as usize,
        )
    }

    /// `UD_EDGES_INVALID` when `mv` takes the UD edges out of their slots.
    pub fn ud_edges_move(&self, ud: u16, mv: Move) -> u16 {
        read_u16(&self.ud_edges_move, ud as usize * MOVE_COUNT + mv as usize)
    }

    pub fn prune_twist_slice(&self, twist: u16, slice: u16) -> u8 {
        self.prune_twist_slice[twist as usize * N_SLICE + slice as usize]
    }

    pub fn prune_flip_slice(&self, flip: u16, slice: u16) -> u8 {
        self.prune_flip_slice[flip as usize * N_SLICE + slice as usize]
    }

    pub fn prune_corners(&self, corners: u16, slice_perm: u16) -> u8 {
        self.prune_corners[corners as usize * N_SLICE_PERM + slice_perm as usize]
    }

    pub fn prune_ud_edges(&self, ud: u16, slice_perm: u16) -> u8 {
        self.prune_ud_edges[ud as usize * N_SLICE_PERM + slice_perm as usize]
    }

    /// Exact phase-1 distances. `None` until `load_optimal` maps them.
    pub fn phase1_prun(&self) -> Option<&[u8]> {
        self.phase1_prun.as_deref()
    }

    /// `(name, max depth, unfilled count)` for the four prune tables.
    pub fn prune_summary(&self) -> [(&'static str, u8, usize); 4] {
        [
            ("twist×slice", depth_span(&self.prune_twist_slice)),
            ("flip×slice", depth_span(&self.prune_flip_slice)),
            ("corners×slice", depth_span(&self.prune_corners)),
            ("ud-edges×slice", depth_span(&self.prune_ud_edges)),
        ]
        .map(|(name, (max, unseen))| (name, max, unseen))
    }
}

/// Build any missing two-phase table, then map the directory.
///
/// Does not build `phase1_prun.bin`. Call `load_optimal` for that.
pub fn load_or_generate(dir: &Path) -> io::Result<LoadedTables> {
    fs::create_dir_all(dir)?;
    let mapped = checksums_match(dir);
    if !mapped {
        let built = build_tables()?;
        write_tables(dir, &built)?;
        if !checksums_match(dir) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "wrote tables that do not match their manifest",
            ));
        }
    }
    let tables = map_tables(dir)?;
    let mut files = specs()
        .into_iter()
        .map(|spec| {
            let path = dir.join(spec.name);
            (path.display().to_string(), spec.bytes as u64)
        })
        .collect::<Vec<_>>();
    let manifest_path = dir.join(MANIFEST);
    let manifest_len = fs::metadata(&manifest_path)?.len();
    files.push((manifest_path.display().to_string(), manifest_len));
    let total = files.iter().map(|(_, n)| *n).sum::<u64>();
    if total > TABLE_BUDGET {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tables are {total} bytes, over the {TABLE_BUDGET} byte budget"),
        ));
    }
    Ok(LoadedTables {
        tables,
        mapped,
        files,
    })
}

/// Two-phase tables plus the exact phase-1 prune used by optimal search.
///
/// A directory that already has a valid two-phase manifest only gains
/// `phase1_prun.bin` when that file is missing or its manifest does not match.
pub fn load_optimal(dir: &Path) -> io::Result<LoadedTables> {
    let mut loaded = load_or_generate(dir)?;
    let had_phase1 = phase1_checksum_ok(dir);
    if !had_phase1 {
        let twist = read_u16_table(&dir.join("twist_move.bin"), N_TWIST * MOVE_COUNT)?;
        let flip = read_u16_table(&dir.join("flip_move.bin"), N_FLIP * MOVE_COUNT)?;
        let slice = read_u16_table(&dir.join("slice_move.bin"), N_SLICE * MOVE_COUNT)?;
        let phase1 = build_phase1_prun(&twist, &flip, &slice)?;
        write_phase1(dir, &phase1)?;
    }
    if !phase1_checksum_ok(dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "wrote phase1 prune that does not match its manifest",
        ));
    }
    loaded.tables.phase1_prun = Some(map_file(&dir.join(PHASE1_FILE), N_PHASE1)?);
    loaded
        .files
        .push((dir.join(PHASE1_FILE).display().to_string(), N_PHASE1 as u64));
    let manifest_len = fs::metadata(dir.join(PHASE1_MANIFEST))?.len();
    loaded.files.push((
        dir.join(PHASE1_MANIFEST).display().to_string(),
        manifest_len,
    ));
    let total = loaded.total_bytes();
    if total > TABLE_BUDGET {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tables are {total} bytes, over the {TABLE_BUDGET} byte budget"),
        ));
    }
    loaded.mapped = loaded.mapped && had_phase1;
    Ok(loaded)
}

fn read_u16_table(path: &Path, count: usize) -> io::Result<Vec<u16>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count * 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} has {} bytes, expected {}",
                path.display(),
                bytes.len(),
                count * 2
            ),
        ));
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

fn write_phase1(dir: &Path, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() != N_PHASE1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{PHASE1_FILE} is {} bytes, expected {N_PHASE1}",
                bytes.len()
            ),
        ));
    }
    write_atomic(dir, PHASE1_FILE, bytes)?;
    write_atomic(
        dir,
        PHASE1_MANIFEST,
        &manifest_bytes(PHASE1_VERSION, &[crc32(bytes)]),
    )?;
    Ok(())
}

fn phase1_checksum_ok(dir: &Path) -> bool {
    let Ok(manifest) = fs::read(dir.join(PHASE1_MANIFEST)) else {
        return false;
    };
    if manifest.len() != 12 || &manifest[..4] != MANIFEST_MAGIC {
        return false;
    }
    let version = u32::from_le_bytes(manifest[4..8].try_into().unwrap());
    if version != PHASE1_VERSION {
        return false;
    }
    let Ok(bytes) = fs::read(dir.join(PHASE1_FILE)) else {
        return false;
    };
    if bytes.len() != N_PHASE1 {
        return false;
    }
    let stored = u32::from_le_bytes(manifest[8..12].try_into().unwrap());
    crc32(&bytes) == stored
}

struct Built {
    twist_move: Vec<u16>,
    flip_move: Vec<u16>,
    slice_move: Vec<u16>,
    slice_sorted_move: Vec<u16>,
    corners_move: Vec<u16>,
    ud_edges_move: Vec<u16>,
    prune_twist_slice: Vec<u8>,
    prune_flip_slice: Vec<u8>,
    prune_corners: Vec<u8>,
    prune_ud_edges: Vec<u8>,
}

fn build_tables() -> io::Result<Built> {
    let twist_move = move_table(N_TWIST, CubieCube::with_twist, CubieCube::twist);
    let flip_move = move_table(N_FLIP, CubieCube::with_flip, CubieCube::flip);
    let slice_move = move_table(N_SLICE, CubieCube::with_slice, CubieCube::slice);
    let slice_sorted_move = move_table(
        N_SLICE_SORTED,
        CubieCube::with_slice_sorted,
        CubieCube::slice_sorted,
    );
    let corners_move = move_table(N_CORNERS, CubieCube::with_corners, CubieCube::corners);
    let ud_edges_move = ud_edges_move_table();

    let prune_twist_slice = bfs_product(
        N_TWIST,
        N_SLICE,
        &ALL_MOVES,
        |twist, mv| twist_move[twist * MOVE_COUNT + mv as usize] as usize,
        |slice, mv| slice_move[slice * MOVE_COUNT + mv as usize] as usize,
    );
    let prune_flip_slice = bfs_product(
        N_FLIP,
        N_SLICE,
        &ALL_MOVES,
        |flip, mv| flip_move[flip * MOVE_COUNT + mv as usize] as usize,
        |slice, mv| slice_move[slice * MOVE_COUNT + mv as usize] as usize,
    );
    let prune_corners = bfs_product(
        N_CORNERS,
        N_SLICE_PERM,
        &PHASE2_MOVES,
        |corners, mv| corners_move[corners * MOVE_COUNT + mv as usize] as usize,
        |slice, mv| {
            let next = slice_sorted_move[slice * MOVE_COUNT + mv as usize] as usize;
            assert!(next < N_SLICE_PERM, "phase-2 move left the UD slice");
            next
        },
    );
    let prune_ud_edges = bfs_product(
        N_UD_EDGES,
        N_SLICE_PERM,
        &PHASE2_MOVES,
        |ud, mv| {
            let next = ud_edges_move[ud * MOVE_COUNT + mv as usize];
            assert_ne!(next, UD_EDGES_INVALID, "phase-2 move scattered UD edges");
            next as usize
        },
        |slice, mv| {
            let next = slice_sorted_move[slice * MOVE_COUNT + mv as usize] as usize;
            assert!(next < N_SLICE_PERM, "phase-2 move left the UD slice");
            next
        },
    );

    require_filled("twist×slice", &prune_twist_slice)?;
    require_filled("flip×slice", &prune_flip_slice)?;
    require_filled("corners×slice", &prune_corners)?;
    require_filled("ud-edges×slice", &prune_ud_edges)?;

    Ok(Built {
        twist_move,
        flip_move,
        slice_move,
        slice_sorted_move,
        corners_move,
        ud_edges_move,
        prune_twist_slice,
        prune_flip_slice,
        prune_corners,
        prune_ud_edges,
    })
}

fn build_phase1_prun(
    twist_move: &[u16],
    flip_move: &[u16],
    slice_move: &[u16],
) -> io::Result<Vec<u8>> {
    let index = sym_index();
    let mut dist = vec![UNSEEN; N_PHASE1];
    let mut queue = Vec::with_capacity(1 << 16);
    dist[0] = 0;
    queue.push(0u32);
    let mut head = 0usize;
    let mut filled = 1usize;
    let mut reported = 0u8;
    while head < queue.len() {
        let state = queue[head] as usize;
        head += 1;
        let class = state / N_TWIST;
        let twist = state % N_TWIST;
        let depth = dist[state];
        if depth > reported {
            reported = depth;
            eprintln!("phase1 depth {depth}, filled {filled}");
        }
        let flip = index.rep_flip[class] as usize;
        let slice = index.rep_slice[class] as usize;
        for m in 0..MOVE_COUNT {
            let twist1 = twist_move[twist * MOVE_COUNT + m] as usize;
            let flip1 = flip_move[flip * MOVE_COUNT + m] as usize;
            let slice1 = slice_move[slice * MOVE_COUNT + m] as usize;
            let fs1 = slice1 * 2048 + flip1;
            let class1 = index.classidx[fs1] as usize;
            let sym1 = index.sym[fs1] as usize;
            let twist_c = index.twist_conj[twist1 * N_SYM + sym1] as usize;
            let idx1 = class1 * N_TWIST + twist_c;
            if dist[idx1] != UNSEEN {
                continue;
            }
            let next_depth = depth + 1;
            dist[idx1] = next_depth;
            queue.push(idx1 as u32);
            filled += 1;
            let mask = index.stab[class1];
            if mask == 1 {
                continue;
            }
            for k in 1..N_SYM {
                if (mask >> k) & 1 == 0 {
                    continue;
                }
                let twist2 = index.twist_conj[twist_c * N_SYM + k] as usize;
                let idx2 = class1 * N_TWIST + twist2;
                if dist[idx2] == UNSEEN {
                    dist[idx2] = next_depth;
                    queue.push(idx2 as u32);
                    filled += 1;
                }
            }
        }
    }
    if filled != N_PHASE1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("phase1 prune filled {filled} of {N_PHASE1}"),
        ));
    }
    eprintln!("phase1 complete at depth {reported}, filled {filled}");
    Ok(dist)
}

fn move_table(n: usize, make: fn(u16) -> CubieCube, read: fn(&CubieCube) -> u16) -> Vec<u16> {
    let mut table = vec![0u16; n * MOVE_COUNT];
    for coord in 0..n {
        let cube = make(coord as u16);
        for mv in ALL_MOVES {
            let mut next = cube;
            next.apply(mv);
            table[coord * MOVE_COUNT + mv as usize] = read(&next);
        }
    }
    table
}

fn ud_edges_move_table() -> Vec<u16> {
    let mut table = vec![UD_EDGES_INVALID; N_UD_EDGES * MOVE_COUNT];
    for coord in 0..N_UD_EDGES {
        let cube = CubieCube::with_ud_edges(coord as u16);
        for mv in ALL_MOVES {
            if !phase2_move(mv) {
                continue;
            }
            let mut next = cube;
            next.apply(mv);
            table[coord * MOVE_COUNT + mv as usize] = next
                .ud_edges()
                .expect("phase-2 move keeps UD edges in their slots");
        }
    }
    table
}

fn bfs_product(
    n_a: usize,
    n_b: usize,
    moves: &[Move],
    step_a: impl Fn(usize, Move) -> usize,
    step_b: impl Fn(usize, Move) -> usize,
) -> Vec<u8> {
    let n = n_a * n_b;
    let mut dist = vec![UNSEEN; n];
    let mut queue = std::collections::VecDeque::new();
    dist[0] = 0;
    queue.push_back(0usize);
    while let Some(state) = queue.pop_front() {
        let depth = dist[state];
        let a = state / n_b;
        let b = state % n_b;
        for &mv in moves {
            let next = step_a(a, mv) * n_b + step_b(b, mv);
            if dist[next] == UNSEEN {
                dist[next] = depth + 1;
                queue.push_back(next);
            }
        }
    }
    dist
}

fn require_filled(name: &str, dist: &[u8]) -> io::Result<()> {
    let unseen = dist.iter().filter(|&&d| d == UNSEEN).count();
    if unseen != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{name} prune left {unseen} states unfilled"),
        ));
    }
    Ok(())
}

fn write_tables(dir: &Path, built: &Built) -> io::Result<()> {
    let files: [(&str, Vec<u8>); 10] = [
        ("twist_move.bin", u16_bytes(&built.twist_move)),
        ("flip_move.bin", u16_bytes(&built.flip_move)),
        ("slice_move.bin", u16_bytes(&built.slice_move)),
        ("slice_sorted_move.bin", u16_bytes(&built.slice_sorted_move)),
        ("corners_move.bin", u16_bytes(&built.corners_move)),
        ("ud_edges_move.bin", u16_bytes(&built.ud_edges_move)),
        ("prune_twist_slice.bin", built.prune_twist_slice.clone()),
        ("prune_flip_slice.bin", built.prune_flip_slice.clone()),
        ("prune_corners.bin", built.prune_corners.clone()),
        ("prune_ud_edges.bin", built.prune_ud_edges.clone()),
    ];
    for (name, bytes) in &files {
        let expected = specs()
            .into_iter()
            .find(|spec| spec.name == *name)
            .map(|spec| spec.bytes)
            .expect("table name");
        if bytes.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{name} is {} bytes, expected {expected}", bytes.len()),
            ));
        }
        write_atomic(dir, name, bytes)?;
    }
    let crcs = files
        .iter()
        .map(|(_, bytes)| crc32(bytes))
        .collect::<Vec<_>>();
    write_atomic(dir, MANIFEST, &manifest_bytes(FORMAT_VERSION, &crcs))?;
    Ok(())
}

fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let tmp = dir.join(format!("{name}.tmp"));
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(tmp, dir.join(name))?;
    Ok(())
}

fn u16_bytes(data: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    for value in data {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn checksums_match(dir: &Path) -> bool {
    let Ok(manifest) = fs::read(dir.join(MANIFEST)) else {
        return false;
    };
    let specs = specs();
    let expected_len = 8 + 4 * specs.len();
    if manifest.len() != expected_len || &manifest[..4] != MANIFEST_MAGIC {
        return false;
    }
    let version = u32::from_le_bytes(manifest[4..8].try_into().unwrap());
    if version != FORMAT_VERSION {
        return false;
    }
    for (i, spec) in specs.iter().enumerate() {
        let Ok(bytes) = fs::read(dir.join(spec.name)) else {
            return false;
        };
        if bytes.len() != spec.bytes {
            return false;
        }
        let stored = u32::from_le_bytes(manifest[8 + i * 4..12 + i * 4].try_into().unwrap());
        if crc32(&bytes) != stored {
            return false;
        }
    }
    true
}

fn manifest_bytes(version: u32, crcs: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 4 * crcs.len());
    out.extend_from_slice(MANIFEST_MAGIC);
    out.extend_from_slice(&version.to_le_bytes());
    for crc in crcs {
        out.extend_from_slice(&crc.to_le_bytes());
    }
    out
}

/// IEEE CRC-32.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn map_tables(dir: &Path) -> io::Result<Tables> {
    if !checksums_match(dir) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "table checksum or format version does not match",
        ));
    }
    let mut maps = Vec::with_capacity(10);
    for spec in specs() {
        maps.push(map_file(&dir.join(spec.name), spec.bytes)?);
    }
    let mut maps = maps.into_iter();
    Ok(Tables {
        twist_move: maps.next().unwrap(),
        flip_move: maps.next().unwrap(),
        slice_move: maps.next().unwrap(),
        slice_sorted_move: maps.next().unwrap(),
        corners_move: maps.next().unwrap(),
        ud_edges_move: maps.next().unwrap(),
        prune_twist_slice: maps.next().unwrap(),
        prune_flip_slice: maps.next().unwrap(),
        prune_corners: maps.next().unwrap(),
        prune_ud_edges: maps.next().unwrap(),
        phase1_prun: None,
    })
}

fn map_file(path: &Path, expected: usize) -> io::Result<Mmap> {
    let file = File::open(path)?;
    let len = file.metadata()?.len() as usize;
    if len != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} has {len} bytes, expected {expected}", path.display()),
        ));
    }
    // Safety: the table file stays read-only for the life of the map.
    unsafe { Mmap::map(&file) }
}

fn read_u16(bytes: &[u8], index: usize) -> u16 {
    let start = index * 2;
    u16::from_le_bytes([bytes[start], bytes[start + 1]])
}

fn depth_span(bytes: &[u8]) -> (u8, usize) {
    let mut max = 0u8;
    let mut unseen = 0usize;
    for &depth in bytes {
        if depth == UNSEEN {
            unseen += 1;
        } else if depth > max {
            max = depth;
        }
    }
    (max, unseen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_ieee_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
