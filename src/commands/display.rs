use super::{BuiltinCommand, CommandInvocation};
use crate::app::Action;
use crate::icons::IconMode;
use crate::status_line::UiPreset;
use crate::theme::{BorderStyle, DynamicFileColorPolicy, SeparatorStyle};

pub(super) fn display_action(
    command: BuiltinCommand,
    invocation: &CommandInvocation,
) -> Result<Action, String> {
    use BuiltinCommand::*;

    Ok(match command {
        LayoutSet => Action::SetStylePreset(UiPreset::parse(single_argument(
            invocation,
            ":layout <framed|borderless|compact>",
        )?)?),
        BorderSet => Action::SetBorderStyle(BorderStyle::parse(single_argument(
            invocation,
            ":border <rounded|plain|double|thick|simple>",
        )?)?),
        SeparatorSet => Action::SetSeparatorStyle(SeparatorStyle::parse(single_argument(
            invocation,
            ":separator <rounded|plain|double|thick|simple|dashed|off>",
        )?)?),
        PaletteUse => {
            Action::UsePalette(single_argument(invocation, ":colorscheme <theme-name>")?.to_owned())
        }
        SyntaxUse => {
            Action::UseSyntax(single_argument(invocation, ":syntax <syntax-name|off>")?.to_owned())
        }
        DisplayStyleUse => {
            Action::UseDisplayStyle(single_argument(invocation, ":style <style-name>")?.to_owned())
        }
        IconsSet => Action::SetIconMode(IconMode::parse(single_argument(
            invocation,
            "icons.set usage: :icons <nerd_font|unicode|none>",
        )?)?),
        NameColorsSet => name_colors(invocation)?,
        FileColorPolicySet => Action::SetDynamicFileColorPolicy(DynamicFileColorPolicy::parse(
            single_argument(invocation, ":file-color-policy <nearest|original>")?,
        )?),
        ThemeUse => Action::UseTheme(
            single_argument(invocation, "theme.use usage: :theme <theme-name>")?.to_owned(),
        ),
        _ => unreachable!("dispatcher routed a non-display command to display_action"),
    })
}

pub(super) fn fixed_argument_values(
    command: BuiltinCommand,
    arguments: &[&str],
) -> &'static [&'static str] {
    match (command, arguments) {
        (BuiltinCommand::LayoutSet, []) => &["framed", "borderless", "compact"],
        (BuiltinCommand::BorderSet, []) => &["rounded", "plain", "double", "thick", "simple"],
        (BuiltinCommand::SeparatorSet, []) => &[
            "rounded", "plain", "double", "thick", "simple", "dashed", "off",
        ],
        (BuiltinCommand::IconsSet, []) => &["nerd_font", "unicode", "none"],
        (BuiltinCommand::NameColorsSet, []) => &["on", "off"],
        (BuiltinCommand::FileColorPolicySet, []) => &["nearest", "original"],
        _ => &[],
    }
}

fn name_colors(invocation: &CommandInvocation) -> Result<Action, String> {
    match invocation.arguments.as_slice() {
        [value] if value == "on" => Ok(Action::SetNameColors(true)),
        [value] if value == "off" => Ok(Action::SetNameColors(false)),
        _ => Err("name_colors.set usage: :name-colors <on|off>".to_owned()),
    }
}

fn single_argument<'a>(invocation: &'a CommandInvocation, usage: &str) -> Result<&'a str, String> {
    match invocation.arguments.as_slice() {
        [argument] => Ok(argument),
        _ => Err(usage.to_owned()),
    }
}
