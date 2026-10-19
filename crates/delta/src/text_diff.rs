/*
Diff Match and Patch
Copyright 2006 Google Inc.
http://code.google.com/p/google-diff-match-patch/
fast-diff Copyright 2014-2023 Jason Chen

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

Modified: Rust UTF-16 port of fast-diff for quillai_delta, 2026.
*/

use crate::{CursorHint, CursorRange};

type Edit = (i8, Vec<u16>);

fn high(c: u16) -> bool {
    (0xd800..=0xdbff).contains(&c)
}
fn low(c: u16) -> bool {
    (0xdc00..=0xdfff).contains(&c)
}

fn prefix(a: &[u16], b: &[u16]) -> usize {
    let mut n = a.iter().zip(b).take_while(|(a, b)| a == b).count();
    if n > 0 && high(a[n - 1]) {
        n -= 1;
    }
    n
}

fn suffix(a: &[u16], b: &[u16]) -> usize {
    let mut n = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    if n > 0 && low(a[a.len() - n]) {
        n -= 1;
    }
    n
}

fn find(a: &[u16], b: &[u16]) -> Option<usize> {
    if b.is_empty() {
        Some(0)
    } else {
        a.windows(b.len()).position(|v| v == b)
    }
}

fn concat(a: &[u16], b: &[u16]) -> Vec<u16> {
    [a, b].concat()
}

fn main_diff(a: &[u16], b: &[u16], fix: bool) -> Vec<Edit> {
    if a == b {
        return if a.is_empty() {
            Vec::new()
        } else {
            vec![(0, a.to_vec())]
        };
    }
    let p = prefix(a, b);
    let s = suffix(&a[p..], &b[p..]);
    let mut out = compute(&a[p..a.len() - s], &b[p..b.len() - s]);
    if p > 0 {
        out.insert(0, (0, a[..p].to_vec()));
    }
    if s > 0 {
        out.push((0, a[a.len() - s..].to_vec()));
    }
    merge(&mut out, fix);
    out
}

fn half_seed(long: &[u16], short: &[u16], i: usize) -> Option<[Vec<u16>; 5]> {
    let seed = &long[i..i + long.len() / 4];
    let mut start = 0;
    let mut best = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    while start <= short.len() {
        let Some(offset) = find(&short[start..], seed) else {
            break;
        };
        let j = start + offset;
        let p = prefix(&long[i..], &short[j..]);
        let s = suffix(&long[..i], &short[..j]);
        if best[4].len() < p + s {
            best = [
                long[..i - s].to_vec(),
                long[i + p..].to_vec(),
                short[..j - s].to_vec(),
                short[j + p..].to_vec(),
                short[j - s..j + p].to_vec(),
            ];
        }
        start = j + 1;
    }
    (best[4].len() * 2 >= long.len()).then_some(best)
}

fn compute(a: &[u16], b: &[u16]) -> Vec<Edit> {
    if a.is_empty() {
        return vec![(1, b.to_vec())];
    }
    if b.is_empty() {
        return vec![(-1, a.to_vec())];
    }
    let (long, short, kind) = if a.len() > b.len() {
        (a, b, -1)
    } else {
        (b, a, 1)
    };
    if let Some(i) = find(long, short) {
        return vec![
            (kind, long[..i].to_vec()),
            (0, short.to_vec()),
            (kind, long[i + short.len()..].to_vec()),
        ];
    }
    if short.len() == 1 {
        return vec![(-1, a.to_vec()), (1, b.to_vec())];
    }
    if long.len() >= 4 && short.len() * 2 >= long.len() {
        let h1 = half_seed(long, short, long.len().div_ceil(4));
        let h2 = half_seed(long, short, long.len().div_ceil(2));
        let hm = match (h1, h2) {
            (Some(x), Some(y)) => Some(if x[4].len() > y[4].len() { x } else { y }),
            (x, y) => x.or(y),
        };
        if let Some([la, lb, sa, sb, mid]) = hm {
            let (aa, ab, ba, bb) = if a.len() > b.len() {
                (la, lb, sa, sb)
            } else {
                (sa, sb, la, lb)
            };
            let mut out = main_diff(&aa, &ba, false);
            out.push((0, mid));
            out.extend(main_diff(&ab, &bb, false));
            return out;
        }
    }
    bisect(a, b)
}

fn split(a: &[u16], b: &[u16], x: usize, y: usize) -> Vec<Edit> {
    let mut out = main_diff(&a[..x], &b[..y], false);
    out.extend(main_diff(&a[x..], &b[y..], false));
    out
}

fn bisect(a: &[u16], b: &[u16]) -> Vec<Edit> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = (n + m + 1) / 2;
    let size = (2 * max) as usize;
    let mut v1 = vec![-1; size + 2];
    let mut v2 = v1.clone();
    v1[(max + 1) as usize] = 0;
    v2[(max + 1) as usize] = 0;
    let delta = n - m;
    let front = delta % 2 != 0;
    let (mut s1, mut e1, mut s2, mut e2) = (0, 0, 0, 0);
    for d in 0..max {
        let mut k = -d + s1;
        while k <= d - e1 {
            let off = (max + k) as usize;
            let mut x = if k == -d || (k != d && v1[off - 1] < v1[off + 1]) {
                v1[off + 1]
            } else {
                v1[off - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v1[off] = x;
            if x > n {
                e1 += 2;
            } else if y > m {
                s1 += 2;
            } else if front {
                let off2 = max + delta - k;
                if off2 >= 0
                    && off2 < size as isize
                    && v2[off2 as usize] != -1
                    && x >= n - v2[off2 as usize]
                {
                    return split(a, b, x as usize, y as usize);
                }
            }
            k += 2;
        }
        let mut k = -d + s2;
        while k <= d - e2 {
            let off = (max + k) as usize;
            let mut x = if k == -d || (k != d && v2[off - 1] < v2[off + 1]) {
                v2[off + 1]
            } else {
                v2[off - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[(n - x - 1) as usize] == b[(m - y - 1) as usize] {
                x += 1;
                y += 1;
            }
            v2[off] = x;
            if x > n {
                e2 += 2;
            } else if y > m {
                s2 += 2;
            } else if !front {
                let off1 = max + delta - k;
                if off1 >= 0 && off1 < size as isize && v1[off1 as usize] != -1 {
                    let x1 = v1[off1 as usize];
                    let y1 = max + x1 - off1;
                    if x1 >= n - x {
                        return split(a, b, x1 as usize, y1 as usize);
                    }
                }
            }
            k += 2;
        }
    }
    vec![(-1, a.to_vec()), (1, b.to_vec())]
}

fn merge(d: &mut Vec<Edit>, fix: bool) {
    d.push((0, Vec::new()));
    let (mut p, mut deletes, mut inserts) = (0, 0, 0);
    let (mut del, mut ins) = (Vec::new(), Vec::new());
    while p < d.len() {
        if p < d.len() - 1 && d[p].1.is_empty() {
            d.remove(p);
            continue;
        }
        match d[p].0 {
            1 => {
                inserts += 1;
                ins.extend(&d[p].1);
                p += 1;
            }
            -1 => {
                deletes += 1;
                del.extend(&d[p].1);
                p += 1;
            }
            _ => {
                let mut prev = p as isize - inserts as isize - deletes as isize - 1;
                if fix {
                    if prev >= 0 && d[prev as usize].1.last().copied().is_some_and(high) {
                        let stray = d[prev as usize].1.pop().expect("last unit");
                        del.insert(0, stray);
                        ins.insert(0, stray);
                        if d[prev as usize].1.is_empty() {
                            d.remove(prev as usize);
                            p -= 1;
                            let mut k = prev - 1;
                            if k >= 0 && d[k as usize].0 == 1 {
                                inserts += 1;
                                ins = concat(&d[k as usize].1, &ins);
                                k -= 1;
                            }
                            if k >= 0 && d[k as usize].0 == -1 {
                                deletes += 1;
                                del = concat(&d[k as usize].1, &del);
                                k -= 1;
                            }
                            prev = k;
                        }
                    }
                    if d[p].1.first().copied().is_some_and(low) {
                        let stray = d[p].1.remove(0);
                        del.push(stray);
                        ins.push(stray);
                    }
                }
                if p < d.len() - 1 && d[p].1.is_empty() {
                    d.remove(p);
                    continue;
                }
                if !del.is_empty() || !ins.is_empty() {
                    if !del.is_empty() && !ins.is_empty() {
                        let n = prefix(&ins, &del);
                        if n > 0 {
                            if prev >= 0 {
                                d[prev as usize].1.extend(&ins[..n]);
                            } else {
                                d.insert(0, (0, ins[..n].to_vec()));
                                p += 1;
                            }
                            ins.drain(..n);
                            del.drain(..n);
                        }
                        let n = suffix(&ins, &del);
                        if n > 0 {
                            d[p].1 = concat(&ins[ins.len() - n..], &d[p].1);
                            ins.truncate(ins.len() - n);
                            del.truncate(del.len() - n);
                        }
                    }
                    let start = p - inserts - deletes;
                    let mut edits = Vec::new();
                    if !del.is_empty() {
                        edits.push((-1, std::mem::take(&mut del)));
                    }
                    if !ins.is_empty() {
                        edits.push((1, std::mem::take(&mut ins)));
                    }
                    let n = edits.len();
                    d.splice(start..p, edits);
                    p = start + n;
                }
                if p > 0 && d[p - 1].0 == 0 {
                    let eq = d.remove(p).1;
                    d[p - 1].1.extend(eq);
                } else {
                    p += 1;
                }
                inserts = 0;
                deletes = 0;
                del.clear();
                ins.clear();
            }
        }
    }
    if d.last().is_some_and(|x| x.1.is_empty()) {
        d.pop();
    }
    let mut changed = false;
    let mut p = 1;
    while p + 1 < d.len() {
        if d[p - 1].0 == 0 && d[p + 1].0 == 0 {
            if d[p].1.ends_with(&d[p - 1].1) {
                let n = d[p].1.len() - d[p - 1].1.len();
                d[p].1 = concat(&d[p - 1].1, &d[p].1[..n]);
                d[p + 1].1 = concat(&d[p - 1].1, &d[p + 1].1);
                d.remove(p - 1);
                changed = true;
            } else if d[p].1.starts_with(&d[p + 1].1) {
                let eq = d[p + 1].1.clone();
                d[p - 1].1.extend(&eq);
                d[p].1 = concat(&d[p].1[eq.len()..], &eq);
                d.remove(p + 1);
                changed = true;
            }
        }
        p += 1;
    }
    if changed {
        merge(d, fix);
    }
}

fn overlap(a: &[u16], b: &[u16]) -> usize {
    (1..=a.len().min(b.len()))
        .rev()
        .find(|&n| a[a.len() - n..] == b[..n])
        .unwrap_or(0)
}

fn whitespace(c: u16) -> bool {
    matches!(c, 0x0009..=0x000d | 0x0020 | 0x00a0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff)
}

fn score(a: &[u16], b: &[u16]) -> usize {
    let (Some(&x), Some(&y)) = (a.last(), b.first()) else {
        return 6;
    };
    let nonalpha = |c: u16| !matches!(c, 65..=90 | 97..=122 | 48..=57);
    let (n1, n2, w1, w2) = (nonalpha(x), nonalpha(y), whitespace(x), whitespace(y));
    let (l1, l2) = (matches!(x, 10 | 13), matches!(y, 10 | 13));
    let blank1 = a.ends_with(&[10, 10]) || a.ends_with(&[10, 13, 10]);
    let blank2 = b.starts_with(&[10, 10])
        || b.starts_with(&[10, 13, 10])
        || b.starts_with(&[13, 10, 10])
        || b.starts_with(&[13, 10, 13, 10]);
    if (l1 && blank1) || (l2 && blank2) {
        5
    } else if l1 || l2 {
        4
    } else if n1 && !w1 && w2 {
        3
    } else if w1 || w2 {
        2
    } else if n1 || n2 {
        1
    } else {
        0
    }
}

fn lossless(d: &mut Vec<Edit>) {
    let mut p = 1;
    while p + 1 < d.len() {
        if d[p - 1].0 == 0 && d[p + 1].0 == 0 {
            let (mut left, mut edit, mut right) =
                (d[p - 1].1.clone(), d[p].1.clone(), d[p + 1].1.clone());
            let n = suffix(&left, &edit);
            if n > 0 {
                let common = edit[edit.len() - n..].to_vec();
                left.truncate(left.len() - n);
                edit = concat(&common, &edit[..edit.len() - n]);
                right = concat(&common, &right);
            }
            let mut best = (left.clone(), edit.clone(), right.clone());
            let mut best_score = score(&left, &edit) + score(&edit, &right);
            while !edit.is_empty() && !right.is_empty() && edit[0] == right[0] {
                left.push(edit.remove(0));
                edit.push(right.remove(0));
                let s = score(&left, &edit) + score(&edit, &right);
                if s >= best_score {
                    best_score = s;
                    best = (left.clone(), edit.clone(), right.clone());
                }
            }
            if d[p - 1].1 != best.0 {
                if best.0.is_empty() {
                    d.remove(p - 1);
                    p -= 1;
                } else {
                    d[p - 1].1 = best.0;
                }
                d[p].1 = best.1;
                if best.2.is_empty() {
                    d.remove(p + 1);
                    p = p.saturating_sub(1);
                } else {
                    d[p + 1].1 = best.2;
                }
            }
        }
        p += 1;
    }
}

fn semantic(d: &mut Vec<Edit>) {
    let mut changed = false;
    let mut equalities = Vec::new();
    let mut last: Option<Vec<u16>> = None;
    let mut p = 0;
    let (mut i1, mut d1, mut i2, mut d2) = (0, 0, 0, 0);
    while p < d.len() {
        if d[p].0 == 0 {
            equalities.push(p);
            i1 = i2;
            d1 = d2;
            i2 = 0;
            d2 = 0;
            last = Some(d[p].1.clone());
        } else {
            if d[p].0 == 1 {
                i2 += d[p].1.len();
            } else {
                d2 += d[p].1.len();
            }
            if let Some(eq) = last
                .as_ref()
                .filter(|v| !v.is_empty() && v.len() <= i1.max(d1) && v.len() <= i2.max(d2))
            {
                let index = equalities.pop().expect("equality index");
                d.insert(index, (-1, eq.clone()));
                d[index + 1].0 = 1;
                equalities.pop();
                p = equalities.last().map_or(0, |v| v + 1);
                i1 = 0;
                d1 = 0;
                i2 = 0;
                d2 = 0;
                last = None;
                changed = true;
                continue;
            }
        }
        p += 1;
    }
    if changed {
        merge(d, false);
    }
    lossless(d);
    let mut p = 1;
    while p < d.len() {
        if d[p - 1].0 == -1 && d[p].0 == 1 {
            let (del, ins) = (d[p - 1].1.clone(), d[p].1.clone());
            let (n1, n2) = (overlap(&del, &ins), overlap(&ins, &del));
            if n1 >= n2 {
                if n1 * 2 >= del.len() || n1 * 2 >= ins.len() {
                    d.insert(p, (0, ins[..n1].to_vec()));
                    d[p - 1].1 = del[..del.len() - n1].to_vec();
                    d[p + 1].1 = ins[n1..].to_vec();
                    p += 1;
                }
            } else if n2 * 2 >= del.len() || n2 * 2 >= ins.len() {
                d.insert(p, (0, del[..n2].to_vec()));
                d[p - 1] = (1, ins[..ins.len() - n2].to_vec());
                d[p + 1] = (-1, del[n2..].to_vec());
                p += 1;
            }
            p += 1;
        }
        p += 1;
    }
}

fn splice(before: &[u16], old: &[u16], new: &[u16], after: &[u16]) -> Option<Vec<Edit>> {
    if before.last().copied().is_some_and(high) || after.first().copied().is_some_and(low) {
        return None;
    }
    Some(
        [(0, before), (-1, old), (1, new), (0, after)]
            .into_iter()
            .filter(|(_, s)| !s.is_empty())
            .map(|(k, s)| (k, s.to_vec()))
            .collect(),
    )
}

fn slice(text: &[u16], start: f64, end: f64) -> &[u16] {
    let index = |n: f64| {
        let n = if n.is_nan() { 0.0 } else { n.trunc() };
        let size = text.len() as f64;
        if n < 0.0 {
            (size + n).max(0.0) as usize
        } else {
            n.min(size) as usize
        }
    };
    let start = index(start);
    &text[start..index(end).max(start)]
}

fn cursor_diff(a: &[u16], b: &[u16], cursor: CursorHint) -> Option<Vec<Edit>> {
    let (old, new) = match cursor {
        CursorHint::Position(index) => (CursorRange { index, length: 0.0 }, None),
        CursorHint::Selection {
            old_range,
            new_range,
        } => (old_range, new_range),
    };
    if old.length == 0.0 && new.is_none_or(|r| r.length == 0.0) {
        let old_before = slice(a, 0.0, old.index);
        let old_after = slice(a, old.index, f64::INFINITY);
        let new_cursor = old.index + b.len() as f64 - a.len() as f64;
        if new.is_none_or(|r| r.index == new_cursor)
            && !(new_cursor < 0.0 || new_cursor > b.len() as f64)
        {
            let new_before = slice(b, 0.0, new_cursor);
            let new_after = slice(b, new_cursor, f64::INFINITY);
            let p = old.index.min(new_cursor);
            let old_prefix = slice(old_before, 0.0, p);
            if new_after == old_after && old_prefix == slice(new_before, 0.0, p) {
                return splice(
                    old_prefix,
                    slice(old_before, p, f64::INFINITY),
                    slice(new_before, p, f64::INFINITY),
                    old_after,
                );
            }
        }
        if new.is_none_or(|r| r.index == old.index) {
            let new_before = slice(b, 0.0, old.index);
            let new_after = slice(b, old.index, f64::INFINITY);
            let s = (a.len() as f64 - old.index).min(b.len() as f64 - old.index);
            let old_suffix = slice(old_after, old_after.len() as f64 - s, f64::INFINITY);
            let new_suffix = slice(new_after, new_after.len() as f64 - s, f64::INFINITY);
            if new_before == old_before && old_suffix == new_suffix {
                return splice(
                    old_before,
                    slice(old_after, 0.0, old_after.len() as f64 - s),
                    slice(new_after, 0.0, new_after.len() as f64 - s),
                    old_suffix,
                );
            }
        }
    }
    if old.length > 0.0 && new.is_some_and(|r| r.length == 0.0) {
        let before = slice(a, 0.0, old.index);
        let after = slice(a, old.index + old.length, f64::INFINITY);
        let p = before.len();
        let s = after.len();
        if b.len() >= p + s && before == &b[..p] && after == &b[b.len() - s..] {
            return splice(
                before,
                slice(a, p as f64, a.len() as f64 - s as f64),
                &b[p..b.len() - s],
                after,
            );
        }
    }
    None
}

pub(crate) fn diff(a: &[u16], b: &[u16], cursor: Option<CursorHint>) -> Vec<Edit> {
    if a != b {
        if let Some(out) = cursor.and_then(|c| cursor_diff(a, b, c)) {
            return out;
        }
    }
    let mut out = main_diff(a, b, true);
    semantic(&mut out);
    out
}
