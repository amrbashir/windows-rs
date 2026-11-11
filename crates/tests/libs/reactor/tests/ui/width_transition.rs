use super::*;

#[test]
fn width_transition_reverses_settles_clears_and_remounts() -> Result<()> {
    process::run("width-transition", |ui| {
        let reference = ui.element("reference")?;
        let base = unsafe {
            let rect = reference.CurrentBoundingRectangle()?;
            rect.right - rect.left
        };
        let width = || unsafe {
            let element = ui.element("indicator").map_err(|error| {
                windows_core::Error::new(
                    windows_core::HRESULT(0x80004005u32 as i32),
                    error.to_string(),
                )
            })?;
            let rect = element.CurrentBoundingRectangle()?;
            Ok(rect.right - rect.left)
        };
        // Initial width must not animate in from zero or Auto.
        assert_eq!(width()?, base);
        ui.invoke("expand")?;
        ui.wait(
            "width changes through intermediate values",
            width,
            |value| *value > base * 3 / 2 && *value < base * 3,
        )?;
        ui.invoke("contract")?;
        assert!(
            width()? < base * 7 / 2,
            "reversal jumped to the previous target"
        );
        ui.wait("reversal reaches the narrow target", width, |value| {
            *value == base
        })?;

        ui.invoke("expand")?;
        ui.wait("expansion is running before removal", width, |value| {
            *value > base && *value < base * 3
        })?;
        ui.invoke("clear-transition")?;
        ui.wait(
            "removing the transition settles at its target",
            width,
            |value| *value == base * 4,
        )?;
        ui.invoke("contract")?;
        ui.wait("width updates without a transition", width, |value| {
            *value == base
        })?;
        ui.invoke("zero-duration")?;
        ui.invoke("expand")?;
        ui.wait("zero duration applies the target", width, |value| {
            *value == base * 4
        })?;
        ui.invoke("clear-width")?;
        ui.wait("clearing width restores Auto", width, |value| {
            *value == base / 3
        })?;

        ui.invoke("show")?;
        ui.wait("initial width is restored", width, |value| *value == base)?;
        ui.invoke("expand")?;
        ui.wait("animation is running before unmount", width, |value| {
            *value > base && *value < base * 3
        })?;
        ui.invoke("hide")?;
        ui.gone("indicator")?;
        ui.invoke("show")?;
        ui.wait("remount starts at its declared width", width, |value| {
            *value == base
        })?;
        Ok(())
    })
}
