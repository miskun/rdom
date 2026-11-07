//! A unified diff (`diff -u`) between two texts, by line, with three lines
//! of context: what `patch` hands back. Its headers name each file by the
//! path the sheet was parsed from — absolute or relative as the app gave
//! it — so `patch -p0` applies it from that path's base; `git apply`
//! takes it only when that path is relative to the repository. The common head and tail are trimmed first, and the rest
//! is matched by longest common subsequence — edits to a stylesheet change
//! a few lines, so the middle is small.

/// The unified diff of `old` → `new` for the file `path`; empty when they
/// are equal.
pub(crate) fn unified(path: &str, old: &str, new: &str) -> String {
    if old == new {
        return String::new();
    }
    let a: Vec<&str> = old.split_inclusive('\n').collect();
    let b: Vec<&str> = new.split_inclusive('\n').collect();
    let ops = lines_diff(&a, &b);
    let mut out = format!("--- {path}\n+++ {path}\n");
    // Group the edit script into hunks with 3 lines of context.
    const CTX: usize = 3;
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, o)| !matches!(o, Op::Same(..)))
        .map(|(i, _)| i)
        .collect();
    let mut k = 0;
    while k < changed.len() {
        let first = changed[k];
        let mut last = first;
        while k + 1 < changed.len() && changed[k + 1] <= last + 2 * CTX + 1 {
            k += 1;
            last = changed[k];
        }
        k += 1;
        let from = first.saturating_sub(CTX);
        let to = (last + CTX + 1).min(ops.len());
        let (mut a_start, mut b_start) = (0, 0);
        for op in &ops[..from] {
            match op {
                Op::Same(..) => {
                    a_start += 1;
                    b_start += 1;
                }
                Op::Del(_) => a_start += 1,
                Op::Add(_) => b_start += 1,
            }
        }
        let a_len = ops[from..to]
            .iter()
            .filter(|o| !matches!(o, Op::Add(_)))
            .count();
        let b_len = ops[from..to]
            .iter()
            .filter(|o| !matches!(o, Op::Del(_)))
            .count();
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            a_start + usize::from(a_len > 0),
            a_len,
            b_start + usize::from(b_len > 0),
            b_len
        ));
        for op in &ops[from..to] {
            let (sign, line) = match op {
                Op::Same(l) => (' ', *l),
                Op::Del(l) => ('-', *l),
                Op::Add(l) => ('+', *l),
            };
            out.push(sign);
            out.push_str(line);
            if !line.ends_with('\n') {
                out.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    out
}

enum Op<'a> {
    Same(&'a str),
    Del(&'a str),
    Add(&'a str),
}

/// The edit script turning `a` into `b`.
fn lines_diff<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Op<'a>> {
    let head = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    // LCS table over the middle.
    let mut t = vec![vec![0u32; mb.len() + 1]; ma.len() + 1];
    for i in (0..ma.len()).rev() {
        for j in (0..mb.len()).rev() {
            t[i][j] = if ma[i] == mb[j] {
                t[i + 1][j + 1] + 1
            } else {
                t[i + 1][j].max(t[i][j + 1])
            };
        }
    }
    let mut ops: Vec<Op<'a>> = a[..head].iter().map(|l| Op::Same(l)).collect();
    let (mut i, mut j) = (0, 0);
    while i < ma.len() || j < mb.len() {
        if i < ma.len() && j < mb.len() && ma[i] == mb[j] {
            ops.push(Op::Same(ma[i]));
            i += 1;
            j += 1;
        } else if j < mb.len() && (i == ma.len() || t[i][j + 1] > t[i + 1][j]) {
            ops.push(Op::Add(mb[j]));
            j += 1;
        } else {
            ops.push(Op::Del(ma[i]));
            i += 1;
        }
    }
    ops.extend(a[a.len() - tail..].iter().map(|l| Op::Same(l)));
    ops
}

#[cfg(test)]
mod tests {
    use super::unified;

    #[test]
    fn a_changed_line_is_a_hunk_with_context() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\n";
        let new = "a\nb\nc\nd\nE\nf\ng\nh\n";
        let d = unified("x.css", old, new);
        assert_eq!(
            d,
            "--- x.css\n+++ x.css\n@@ -2,7 +2,7 @@\n b\n c\n d\n-e\n+E\n f\n g\n h\n"
        );
        assert_eq!(unified("x.css", old, old), "");
    }

    #[test]
    fn an_added_line_counts_right() {
        let d = unified("x.css", "a {\n}\n", "a {\n  color: red;\n}\n");
        assert_eq!(
            d,
            "--- x.css\n+++ x.css\n@@ -1,2 +1,3 @@\n a {\n+  color: red;\n }\n"
        );
    }
}
