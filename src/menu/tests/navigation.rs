use super::*;

use crate::menu::view::{Label, Panel};

fn back_visuals(
    runner: &HeadlessRunner<MenuAction>,
) -> LogicResult<(&ScreenRectangleVisual, &ScreenTextVisual)> {
    let (panel, _) = runner
        .components::<Panel>()
        .find(|(_, kind)| matches!(kind, Panel::Button(Target::BackToMenu)))
        .ok_or("picker back panel missing")?;
    let (label, _) = runner
        .components::<Label>()
        .find(|(_, kind)| matches!(kind, Label::Button(Target::BackToMenu)))
        .ok_or("picker back label missing")?;
    Ok((runner.component(panel)?, runner.component(label)?))
}

fn assert_back_alpha(runner: &HeadlessRunner<MenuAction>) -> LogicResult<f32> {
    let (panel, label) = back_visuals(runner)?;
    let blend = state(runner).domains_blend;
    assert!((panel.color().alpha() - blend).abs() < 0.00001);
    assert!((label.tint().alpha() - blend).abs() < 0.00001);
    Ok(blend)
}

#[test]
fn picker_back_label_is_centered_in_its_own_button_across_resizes() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Domains)?;
    for (width, height) in [
        (360.0, 560.0),
        (720.0, 720.0),
        (1280.0, 800.0),
        (1920.0, 1080.0),
    ] {
        let viewport = LogicalViewport::new(width, height)?;
        advance(&mut runner, viewport, &[])?;
        let (panel, label) = back_visuals(&runner)?;
        let position = panel.position().to_vec2();
        let size = panel.size().to_vec2();
        let baseline = label.baseline_origin().to_vec2();
        let metrics = label.metrics();
        let center_y = baseline.y() - (metrics.ascent() + metrics.descent()) * 0.5;
        assert!((position.x() + size.x() * 0.5 - width * 0.5).abs() < 0.001);
        assert!((baseline.x() + metrics.advance() * 0.5 - width * 0.5).abs() < 0.001);
        assert!((center_y - position.y() - size.y() * 0.5).abs() < 0.001);
        assert!(baseline.y() - metrics.ascent() >= position.y());
        assert!(baseline.y() - metrics.descent() <= position.y() + size.y());
        assert_eq!(label.text(), "Back to main menu");
        assert_eq!(
            super::super::layout::hit(
                state(&runner),
                Some(PointerSample::new(
                    LogicalScreenPosition::new(width * 0.5, position.y() + size.y() * 0.5),
                    viewport,
                )?),
            ),
            Some(Target::BackToMenu),
        );
    }
    Ok(())
}

#[test]
fn picker_back_fades_with_rows_and_never_teleports_to_a_modal() -> LogicResult {
    let mut runner = runner()?;
    advance(&mut runner, viewport(), &[])?;
    assert_eq!(assert_back_alpha(&runner)?, 0.0);
    click(&mut runner, Target::Domains)?;
    let mut previous = assert_back_alpha(&runner)?;
    assert!(previous > 0.0 && previous < 1.0);
    for _ in 0..30 {
        advance(&mut runner, viewport(), &[])?;
        let alpha = assert_back_alpha(&runner)?;
        assert!(alpha > previous && alpha < 1.0);
        previous = alpha;
    }
    let before = back_visuals(&runner)?.0.position();
    click(&mut runner, Target::BackToMenu)?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    assert!(!state(&runner).targets().contains(&Target::BackToMenu));
    assert!(assert_back_alpha(&runner)? > 0.0);
    assert!(assert_back_alpha(&runner)? < previous);
    assert_eq!(back_visuals(&runner)?.0.position(), before);
    click(&mut runner, Target::Settings)?;
    assert_eq!(state(&runner).overlay, Overlay::Settings);
    assert_eq!(back_visuals(&runner)?.0.position(), before);
    assert_eq!(back_visuals(&runner)?.1.text(), "Back to main menu");
    assert!(state(&runner).targets().contains(&Target::Close));
    assert!(!state(&runner).targets().contains(&Target::BackToMenu));
    for _ in 0..100 {
        advance(&mut runner, viewport(), &[])?;
    }
    let (panel, label) = back_visuals(&runner)?;
    assert_eq!(panel.clip(), ScreenClip::Empty);
    assert_eq!(label.clip(), ScreenClip::Empty);
    Ok(())
}

#[test]
fn picker_back_supports_smooth_hover_keyboard_return_and_reduced_motion() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Domains)?;
    advance(
        &mut runner,
        viewport(),
        &[over(Target::BackToMenu, Overlay::Domains, viewport())],
    )?;
    let emphasis = state(&runner).emphasis[Target::BackToMenu.index()];
    assert!(emphasis > 0.0 && emphasis < 1.0);
    advance(&mut runner, viewport(), &[])?;
    assert!(state(&runner).emphasis[Target::BackToMenu.index()] > emphasis);
    advance(&mut runner, viewport(), &[InputEvent::PointerLeft])?;
    for _ in 0..5 {
        advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    }
    assert_eq!(state(&runner).focus.focused(), Some(Target::BackToMenu));
    let before_return = state(&runner).emphasis[Target::BackToMenu.index()];
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    let fading = state(&runner).emphasis[Target::BackToMenu.index()];
    assert!(fading > 0.0 && fading < before_return);
    click(&mut runner, Target::Settings)?;
    click(&mut runner, Target::ReduceMotion)?;
    click(&mut runner, Target::Close)?;
    assert_eq!(assert_back_alpha(&runner)?, 0.0);
    click(&mut runner, Target::Domains)?;
    assert_eq!(assert_back_alpha(&runner)?, 1.0);
    click(&mut runner, Target::BackToMenu)?;
    assert_eq!(assert_back_alpha(&runner)?, 0.0);
    Ok(())
}
