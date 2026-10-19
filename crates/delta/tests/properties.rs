//! Seeded property tests for compose / transform / transform_position.
//! No external crates: a splitmix64 PRNG and hand-rolled generators.
//! On failure the panic message holds the seed, case index and input Deltas as JSON.

use quillai_delta::{Attributes, Delta, Op, Retain};
use serde_json::{json, Value};

const SEEDS: [u64; 5] = [1, 42, 2908, 0xDEAD_BEEF, 0x5EED_5EED_5EED];
const CASES_PER_SEED: usize = 1000;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform-ish in `0..n` (`n > 0`).
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Text atoms (UTF-16 lengths 1, 1, 1, 2, 1) and an embed (length 1).
const ALPHABET: [&str; 5] = ["a", "b", "\n", "😀", "日"];

fn attrs(rng: &mut Rng, allow_null: bool) -> Option<Attributes> {
    let mut a = Attributes::new();
    for _ in 0..rng.below(3) {
        let (k, v): (&str, Value) = match rng.below(4) {
            0 => ("bold", json!(true)),
            1 => ("italic", json!(true)),
            2 => ("color", json!(["red", "blue"][rng.below(2)])),
            _ if allow_null => ("bold", Value::Null),
            _ => ("size", json!(rng.below(3))),
        };
        a.insert(k.to_string(), v);
    }
    (!a.is_empty()).then_some(a)
}

/// Insert of 1..=3 atoms; returns the op and the atom lengths it adds.
fn insert_op(rng: &mut Rng, lens: &mut Vec<usize>) -> Op {
    let a = attrs(rng, false);
    if rng.below(8) == 0 {
        lens.push(1);
        let embed = json!({"image": "x.png"}).as_object().cloned().unwrap();
        return Op::insert_embed(embed, a);
    }
    let mut s = String::new();
    for _ in 0..1 + rng.below(3) {
        let t = ALPHABET[rng.below(ALPHABET.len())];
        lens.push(t.encode_utf16().count());
        s.push_str(t);
    }
    Op::insert_text(s, a).unwrap()
}

/// Random document (inserts only). Returns it with the UTF-16 length of each atom.
fn gen_doc(rng: &mut Rng) -> (Delta, Vec<usize>) {
    let (mut d, mut lens) = (Delta::new(), Vec::new());
    for _ in 0..rng.below(6) {
        d.push(insert_op(rng, &mut lens));
    }
    (d, lens)
}

/// Random change (<= 8 ops) valid for a document with atoms `lens`: retains and
/// deletes cover whole atoms, so no emoji is cut in half.
fn gen_change(rng: &mut Rng, lens: &[usize]) -> Delta {
    let (mut d, mut pos, mut scratch) = (Delta::new(), 0, Vec::new());
    for _ in 0..1 + rng.below(8) {
        let left = lens.len() - pos;
        let kind = rng.below(3);
        if kind == 0 || left == 0 {
            d.push(insert_op(rng, &mut scratch));
            continue;
        }
        let n = 1 + rng.below(left.min(4));
        let units: usize = lens[pos..pos + n].iter().sum();
        pos += n;
        if kind == 1 {
            d.push(Op::retain(units, attrs(rng, true)).unwrap());
        } else {
            d.push(Op::delete(units).unwrap());
        }
    }
    d
}

fn doc_len(lens: &[usize]) -> usize {
    lens.iter().sum()
}

/// Output must survive a JSON round trip through the strict parser.
fn reparse(what: &str, d: &Delta) -> Result<(), String> {
    match Delta::from_json(&d.to_json()) {
        Ok(back) if &back == d => Ok(()),
        Ok(_) => Err(format!("{what}: re-parse differs")),
        Err(e) => Err(format!("{what}: re-parse failed: {e}")),
    }
}

/// Leading retain length before the first insert (position of a marker insert).
fn marker_pos(d: &Delta) -> Option<usize> {
    let mut at = 0;
    for op in &d.ops {
        match op {
            Op::Insert { .. } => return Some(at),
            Op::Retain {
                retain: Retain::Count(n),
                ..
            } => at += *n,
            _ => return None,
        }
    }
    None
}

fn check(rng: &mut Rng, case: &mut Vec<(&'static str, Delta)>) -> Result<(), String> {
    let (doc, lens) = gen_doc(rng);
    let a = gen_change(rng, &lens);
    let b = gen_change(rng, &lens);
    case.extend([("doc", doc.clone()), ("a", a.clone()), ("b", b.clone())]);

    let c = |x: &Delta, y: &Delta| x.compose(y).map_err(|e| format!("compose failed: {e}"));
    let a_t = a.transform(&b, true);
    let b_t = b.transform(&a, false);
    let left = c(&c(&doc, &a)?, &a_t)?;
    let right = c(&c(&doc, &b)?, &b_t)?;
    for (what, d) in [
        ("a'", &a_t),
        ("b'", &b_t),
        ("left", &left),
        ("right", &right),
    ] {
        reparse(what, d)?;
    }
    if left != right {
        return Err(format!(
            "no convergence:\n left  {}\n right {}",
            left.to_json(),
            right.to_json()
        ));
    }
    reparse("doc∘a", &c(&doc, &a)?)?;
    reparse("doc∘b", &c(&doc, &b)?)?;

    // transform_position agrees with transforming a marker insert at the same index.
    for _ in 0..4 {
        let i = rng.below(doc_len(&lens) + 1);
        let mut m = Delta::new();
        if let Some(r) = Op::retain(i, None) {
            m.push(r);
        }
        m.push(Op::insert_text("X", None).unwrap());
        for cursor_priority in [false, true] {
            // Cursor priority is the opposite of the transform's `priority` (self wins).
            let want = marker_pos(&a.transform(&m, !cursor_priority));
            let got = a.transform_position(i, cursor_priority);
            if want != Some(got) {
                return Err(format!(
                    "transform_position({i}, {cursor_priority}) = {got}, marker transform = {want:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn compose_transform_properties() {
    let mut total = 0;
    for seed in SEEDS {
        let mut rng = Rng(seed);
        for index in 0..CASES_PER_SEED {
            let mut case = Vec::new();
            if let Err(msg) = check(&mut rng, &mut case) {
                let inputs: Vec<String> = case
                    .iter()
                    .map(|(n, d)| format!("  {n}: {}", d.to_json()))
                    .collect();
                panic!(
                    "property failed: seed={seed} case={index}\n{msg}\n{}",
                    inputs.join("\n")
                );
            }
            total += 1;
        }
    }
    assert_eq!(total, SEEDS.len() * CASES_PER_SEED);
    println!("{total} property cases passed");
}
