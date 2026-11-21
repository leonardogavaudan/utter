use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    pub fn intersection(&self, other: &Frame) -> Option<Frame> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > left && bottom > top).then_some(Frame {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }

    fn contains(&self, other: &Frame) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }

    fn distance_to(&self, other: &Frame) -> f64 {
        let dx = (other.x - (self.x + self.width))
            .max(self.x - (other.x + other.width))
            .max(0.0);
        let dy = (other.y - (self.y + self.height))
            .max(self.y - (other.y + other.height))
            .max(0.0);
        dx.hypot(dy)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub frame: Frame,
}

pub fn nearby_text(runs: Vec<TextRun>, focus: Option<Frame>, max_chars: usize) -> Option<String> {
    let mut seen = HashSet::new();
    let mut candidates: Vec<TextRun> = runs
        .into_iter()
        .map(|run| TextRun {
            text: run.text.trim().to_owned(),
            frame: run.frame,
        })
        .filter(|run| !run.text.is_empty())
        .filter(|run| !focus.is_some_and(|focus| focus.contains(&run.frame)))
        .filter(|run| seen.insert(run.text.clone()))
        .collect();

    if let Some(focus) = focus {
        candidates.sort_by(|a, b| {
            a.frame
                .distance_to(&focus)
                .total_cmp(&b.frame.distance_to(&focus))
        });
    } else {
        sort_in_reading_order(&mut candidates);
    }

    let mut budget = max_chars;
    let mut selected = Vec::new();
    for mut run in candidates {
        let length = run.text.chars().count();
        if length + 1 > budget {
            if budget > 1 {
                run.text = run.text.chars().take(budget - 1).collect();
                selected.push(run);
            }
            break;
        }
        budget -= length + 1;
        selected.push(run);
    }
    if selected.is_empty() {
        return None;
    }

    sort_in_reading_order(&mut selected);
    let mut text = String::new();
    let mut previous: Option<Frame> = None;
    for run in selected {
        if let Some(previous) = previous {
            let same_line =
                (run.frame.y - previous.y).abs() < previous.height.min(run.frame.height) / 2.0;
            text.push(if same_line { ' ' } else { '\n' });
        }
        text.push_str(&run.text);
        previous = Some(run.frame);
    }
    Some(text)
}

fn sort_in_reading_order(runs: &mut [TextRun]) {
    runs.sort_by(|a, b| {
        a.frame
            .y
            .total_cmp(&b.frame.y)
            .then(a.frame.x.total_cmp(&b.frame.x))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, x: f64, y: f64) -> TextRun {
        TextRun {
            text: text.to_owned(),
            frame: Frame {
                x,
                y,
                width: 100.0,
                height: 20.0,
            },
        }
    }

    fn field_at(y: f64) -> Frame {
        Frame {
            x: 0.0,
            y,
            width: 800.0,
            height: 60.0,
        }
    }

    #[test]
    fn keeps_the_text_closest_to_the_cursor_and_returns_it_in_reading_order() {
        let runs = vec![
            run("far away heading", 0.0, 0.0),
            run("older message", 0.0, 400.0),
            run("latest message", 0.0, 700.0),
        ];
        let text = nearby_text(runs, Some(field_at(760.0)), 30).expect("has text");
        assert_eq!(text, "older message\nlatest message");
    }

    #[test]
    fn prefers_text_above_the_field_over_a_sidebar_beside_it() {
        let runs = vec![
            run("sidebar thread", -400.0, 770.0),
            run("message above", 0.0, 700.0),
        ];
        let text = nearby_text(runs, Some(field_at(760.0)), 15).expect("has text");
        assert_eq!(text, "message above");
    }

    #[test]
    fn skips_text_inside_the_focused_field_and_duplicates() {
        let runs = vec![
            run("Reply", 0.0, 100.0),
            run("Reply", 0.0, 200.0),
            run("draft in the field", 10.0, 770.0),
        ];
        let text = nearby_text(runs, Some(field_at(760.0)), 500).expect("has text");
        assert_eq!(text, "Reply");
    }

    #[test]
    fn joins_runs_on_the_same_line_with_a_space() {
        let runs = vec![
            run("Priyanka", 0.0, 100.0),
            run("12:04", 120.0, 102.0),
            run("Ship it", 0.0, 130.0),
        ];
        let text = nearby_text(runs, None, 500).expect("has text");
        assert_eq!(text, "Priyanka 12:04\nShip it");
    }

    #[test]
    fn truncates_the_last_run_to_fit_the_budget() {
        let runs = vec![run("abcdefghij", 0.0, 0.0)];
        assert_eq!(nearby_text(runs, None, 5).as_deref(), Some("abcd"));
        assert_eq!(nearby_text(vec![run("   ", 0.0, 0.0)], None, 50), None);
    }

    #[test]
    fn intersects_frames() {
        let window = Frame {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let partly_outside = Frame {
            x: 50.0,
            y: 80.0,
            width: 100.0,
            height: 100.0,
        };
        let outside = Frame {
            x: 0.0,
            y: 200.0,
            width: 10.0,
            height: 10.0,
        };
        assert_eq!(
            window.intersection(&partly_outside),
            Some(Frame {
                x: 50.0,
                y: 80.0,
                width: 50.0,
                height: 20.0
            })
        );
        assert_eq!(window.intersection(&outside), None);
    }
}
