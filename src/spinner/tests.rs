use super::*;

#[test]
fn new_spinner_starts_at_first_frame() {
    let spinner = Spinner::new();
    assert_eq!(spinner.current(), '⠋');
}

#[test]
fn tick_advances_to_next_frame() {
    let mut spinner = Spinner::new();
    spinner.tick();
    assert_eq!(spinner.current(), '⠙');
}

#[test]
fn tick_wraps_around_after_last_frame() {
    let mut spinner = Spinner::new();
    for _ in 0..10 {
        spinner.tick();
    }
    // After 10 ticks on a 10-frame spinner, back to frame 0
    assert_eq!(spinner.current(), '⠋');
}

#[test]
fn current_returns_braille_character() {
    let mut spinner = Spinner::new();
    let expected = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    for &ch in &expected {
        assert_eq!(spinner.current(), ch);
        spinner.tick();
    }
}

#[test]
fn reset_returns_to_first_frame() {
    let mut spinner = Spinner::new();
    spinner.tick();
    spinner.tick();
    spinner.reset();
    assert_eq!(spinner.current(), '⠋');
}
