"""Theme palette definitions for tooling integrations.

This module mirrors the Rust theme registry so external tooling (formatters,
previews, or UI experiments) can reason about available color palettes without
re-implementing the registry logic.
"""

from dataclasses import dataclass
from typing import Dict


@dataclass(frozen=True)
class ThemePalette:
    primary_accent: str
    background: str
    foreground: str
    secondary_accent: str
    alert: str


THEMES: Dict[str, ThemePalette] = {
    "ciapre-dark": ThemePalette(
        primary_accent="#BFB38F",
        background="#262626",
        foreground="#BFB38F",
        secondary_accent="#D99A4E",
        alert="#FF8A8A",
    ),
    "ciapre-blue": ThemePalette(
        primary_accent="#BFB38F",
        background="#383B73",
        foreground="#BFB38F",
        secondary_accent="#BFB38F",
        alert="#FF8A8A",
    ),
}

DEFAULT_THEME = "ciapre-dark"


def available() -> Dict[str, ThemePalette]:
    """Return a copy of the theme map for safe external consumption."""
    return dict(THEMES)
