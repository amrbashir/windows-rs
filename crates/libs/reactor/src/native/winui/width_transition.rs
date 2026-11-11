use super::*;

/// A width policy and its current native clock; at most one clock exists per element.
pub(super) struct WidthTransition {
    duration: Duration,
    pub(super) running: Option<RunningWidthAnimation>,
}

pub(super) struct RunningWidthAnimation {
    storyboard: native::Storyboard,
    target: f64,
}

impl Drop for RunningWidthAnimation {
    fn drop(&mut self) {
        let _ = self.storyboard.Stop();
    }
}

pub(super) struct WidthUpdate {
    element: native::FrameworkElement,
    target: Option<f64>,
    duration: Option<Duration>,
    previous: Option<RunningWidthAnimation>,
}

impl WinUiAdapter {
    pub(super) fn prepare_width_transition(
        &mut self,
        object: ObjectId,
        set: &[Property],
        clear: &[PropertyId],
        initializing: bool,
    ) -> Result<Option<WidthUpdate>, WinUiError> {
        let duration = set.iter().find_map(|property| {
            if property.id == PropertyId::WidthTransition {
                let PropertyValue::Duration(value) = property.value else {
                    unreachable!()
                };
                Some(value)
            } else {
                None
            }
        });
        if let Some(duration) = duration {
            TimeSpan::try_from(duration).map_err(|_| WinUiError::InvalidDuration)?;
            self.width_transitions
                .entry(object)
                .and_modify(|transition| transition.duration = duration)
                .or_insert(WidthTransition {
                    duration,
                    running: None,
                });
        }

        let removing = clear.contains(&PropertyId::WidthTransition);
        let width = set.iter().find_map(|property| {
            if property.id == PropertyId::Width {
                let PropertyValue::F64(value) = property.value else {
                    unreachable!()
                };
                Some(value)
            } else {
                None
            }
        });
        let clearing_width = clear.contains(&PropertyId::Width);
        let Some(transition) = self.width_transitions.get_mut(&object) else {
            return Ok(None);
        };
        let settling = removing || transition.duration.is_zero();
        if initializing
            || (width.is_none() && !clearing_width && !(settling && transition.running.is_some()))
        {
            if removing {
                self.width_transitions.remove(&object);
            }
            return Ok(None);
        }

        let target = if clearing_width {
            None
        } else {
            width.or_else(|| {
                transition
                    .running
                    .as_ref()
                    .map(|animation| animation.target)
            })
        };
        let duration = (!settling && !clearing_width).then_some(transition.duration);
        let previous = transition.running.take();
        if removing {
            self.width_transitions.remove(&object);
        }
        let element = self
            .ui_element(object)?
            .cast::<native::FrameworkElement>()?;
        Ok(Some(WidthUpdate {
            element,
            target,
            duration,
            previous,
        }))
    }
}

impl WidthUpdate {
    pub(super) fn apply(self) -> Result<Option<RunningWidthAnimation>, WinUiError> {
        // Read the animated value before stopping the previous clock. This also makes
        // reversal continuous rather than jumping to the previous destination.
        let from = self.element.Width()?;
        drop(self.previous);
        let Some(target) = self.target else {
            dependency_write(
                &self.element.cast()?,
                ObjectType::Border,
                PropertyId::Width,
                None,
            )
            .unwrap()?
            .apply()?;
            return Ok(None);
        };
        // Commit the final base value so FillBehavior=Stop leaves the intended width.
        self.element.SetWidth(target)?;
        let Some(duration) = self.duration.filter(|_| from.is_finite() && from != target) else {
            return Ok(None);
        };

        let ticks = TimeSpan::try_from(duration)
            .map_err(|_| WinUiError::InvalidDuration)?
            .duration;
        let hours = ticks / 36_000_000_000;
        let minutes = (ticks / 600_000_000) % 60;
        let seconds = (ticks / 10_000_000) % 60;
        let fraction = ticks % 10_000_000;
        let storyboard: native::Storyboard = native::XamlReader::Load(&format!(
            r#"<Storyboard xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation">
                <DoubleAnimation Storyboard.TargetProperty="Width" From="{from}" To="{target}"
                    Duration="{hours}:{minutes:02}:{seconds:02}.{fraction:07}"
                    EnableDependentAnimation="True" FillBehavior="Stop">
                    <DoubleAnimation.EasingFunction>
                        <SineEase EasingMode="EaseOut"/>
                    </DoubleAnimation.EasingFunction>
                </DoubleAnimation>
            </Storyboard>"#
        ))?
        .cast()?;
        let animation = storyboard.Children()?.GetAt(0)?;
        native::Storyboard::SetTarget(&animation, &self.element)?;
        storyboard.Begin()?;
        Ok(Some(RunningWidthAnimation { storyboard, target }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflowing_duration_is_rejected_before_installing_the_policy() {
        let mut adapter = WinUiAdapter::default();
        let object = ObjectId::test(1);
        let result = adapter.prepare_width_transition(
            object,
            &[Property {
                id: PropertyId::WidthTransition,
                value: PropertyValue::Duration(Duration::MAX),
            }],
            &[],
            true,
        );
        assert!(matches!(result, Err(WinUiError::InvalidDuration)));
        assert!(adapter.width_transitions.is_empty());
    }
}
