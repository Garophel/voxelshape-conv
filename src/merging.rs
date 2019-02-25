use crate::AABox;

pub fn merge_touching(boxes: &[AABox]) -> Vec<AABox> {
    let mut work: Vec<AABox> = Vec::new();
    boxes.iter().for_each(|bx| work.push(bx.clone()));

    loop {
        let count = work.len();
        let mut brek = false;

        for i in 0..count {
            for j in 0..count {
                if i == j {
                    continue;
                }

                if touching(&work[i], &work[j]) {
                    let (a, b) = if i < j {
                        (work.remove(j), work.remove(i))
                    } else {
                        (work.remove(i), work.remove(j))
                    };

                    work.push(merge(a, b));

                    brek = true;
                    break;
                }
            }

            if brek {
                break;
            }
        }

        if !brek {
            break work
        }
    }
}

fn merge(mut a: AABox, b: AABox) -> AABox {
    a.0 = a.0.min(b.0);
    a.1 = a.1.min(b.1);
    a.2 = a.2.min(b.2);

    a.3 = a.3.max(b.3);
    a.4 = a.4.max(b.4);
    a.5 = a.5.max(b.5);

    a
}

fn touching(a: &AABox, b: &AABox) -> bool {
    let x_overlap = axis_overlap(a.0, a.3, b.0, b.3);
    let y_overlap = axis_overlap(a.1, a.4, b.1, b.4);
    let z_overlap = axis_overlap(a.2, a.5, b.2, b.5);

    x_overlap && y_overlap && z_overlap
}

fn axis_overlap(va0: f32, va1: f32, vb0: f32, vb1: f32) -> bool {
    vb0 >= va0 && vb0 <= va1 ||
        vb1 >= va0 && vb1 <= va1 ||
        va0 >= vb0 && va0 <= vb1 ||
        va1 >= vb0 && va1 <= vb1
}
