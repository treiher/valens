#!/usr/bin/env python

"""Visualize the currently used 1RM estimation formula as a line diagram (SVG)."""

from __future__ import annotations

import argparse
import math
from pathlib import Path


def calculate_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the current hybrid formula."""
    if reps <= 1.0:
        return weight
    if reps <= 6.0:
        return brzycki_one_rep_max(reps, weight)
    if reps <= 19.0:
        return desgorces_one_rep_max(reps, weight)
    return wathan_one_rep_max(reps, weight)


def brzycki_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the Brzycki formula."""
    return weight * 36.0 / (37.0 - reps)


def desgorces_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the Desgorces formula."""
    return (100.0 * weight) / (83.7677 * math.exp(-0.0338 * reps) + 17.6846)


def epley_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the Epley formula."""
    return weight * (1.0 + reps / 30.0)


def landers_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the Landers formula."""
    return (100.0 * weight) / (101.3 - 2.67123 * reps)


def mayhew_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the Mayhew formula."""
    return (100.0 * weight) / (52.2 + 41.9 * math.exp(-0.055 * reps))


def wathan_one_rep_max(reps: float, weight: float) -> float:
    """Calculate 1RM using the Wathan formula."""
    return (100.0 * weight) / (48.8 + 53.8 * math.exp(-0.075 * reps))


def build_svg(weight: float, reps_max: int, width: int = 900, height: int = 520) -> str:
    margin_left = 70
    margin_right = 20
    margin_top = 40
    margin_bottom = 60
    plot_width = width - margin_left - margin_right
    plot_height = height - margin_top - margin_bottom

    reps_values = [float(r) for r in range(1, reps_max + 1)]
    y_values = [calculate_one_rep_max(reps, weight) for reps in reps_values]
    y_values.extend(brzycki_one_rep_max(reps, weight) for reps in reps_values)
    y_values.extend(desgorces_one_rep_max(reps, weight) for reps in reps_values)
    y_values.extend(epley_one_rep_max(reps, weight) for reps in reps_values)
    y_values.extend(landers_one_rep_max(reps, weight) for reps in reps_values)
    y_values.extend(mayhew_one_rep_max(reps, weight) for reps in reps_values)
    y_values.extend(wathan_one_rep_max(reps, weight) for reps in reps_values)
    y_min = min(y_values)
    y_max = max(y_values)
    y_pad = max((y_max - y_min) * 0.07, 1.0)
    y_min -= y_pad
    y_max += y_pad

    def x_px(reps: float) -> float:
        return margin_left + (reps - 1.0) / (reps_max - 1.0) * plot_width

    def y_px(one_rm: float) -> float:
        return margin_top + (1.0 - (one_rm - y_min) / (y_max - y_min)) * plot_height

    def polyline(values: list[tuple[float, float]]) -> str:
        return " ".join(f"{x_px(x):.2f},{y_px(y):.2f}" for x, y in values)

    current_values = [(r, calculate_one_rep_max(weight, r)) for r in reps_values]
    brzycki_values = [(r, brzycki_one_rep_max(weight, r)) for r in reps_values]
    desgorces_values = [(r, desgorces_one_rep_max(weight, r)) for r in reps_values]
    epley_values = [(r, epley_one_rep_max(weight, r)) for r in reps_values]
    landers_values = [(r, landers_one_rep_max(weight, r)) for r in reps_values]
    mayhew_values = [(r, mayhew_one_rep_max(weight, r)) for r in reps_values]
    wathan_values = [(r, wathan_one_rep_max(weight, r)) for r in reps_values]

    x_ticks = range(1, reps_max + 1, 2)
    y_ticks = 6
    y_step = (y_max - y_min) / y_ticks

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" font-family="sans-serif">',
        '<rect width="100%" height="100%" fill="white" />',
        f'<text x="{width / 2:.0f}" y="24" text-anchor="middle" font-size="18" font-weight="bold">'
        "1RM estimation formula</text>",
        f'<text x="{width / 2:.0f}" y="44" text-anchor="middle" font-size="13" fill="#444">'
        f"weight = {weight:g} kg</text>",
        # Axes
        f'<line x1="{margin_left}" y1="{margin_top + plot_height}" x2="{margin_left + plot_width}" '
        f'y2="{margin_top + plot_height}" stroke="#333" />',
        f'<line x1="{margin_left}" y1="{margin_top}" x2="{margin_left}" '
        f'y2="{margin_top + plot_height}" stroke="#333" />',
    ]

    for tick in x_ticks:
        x = x_px(float(tick))
        parts.append(
            f'<line x1="{x:.2f}" y1="{margin_top + plot_height}" x2="{x:.2f}" '
            f'y2="{margin_top + plot_height + 5}" stroke="#333" />'
        )
        parts.append(
            f'<text x="{x:.2f}" y="{margin_top + plot_height + 22}" text-anchor="middle" '
            f'font-size="12">{tick}</text>'
        )

    for i in range(y_ticks + 1):
        y_value = y_min + i * y_step
        y = y_px(y_value)
        parts.append(
            f'<line x1="{margin_left - 5}" y1="{y:.2f}" x2="{margin_left}" '
            f'y2="{y:.2f}" stroke="#333" />'
        )
        parts.append(
            f'<line x1="{margin_left}" y1="{y:.2f}" x2="{margin_left + plot_width}" y2="{y:.2f}" '
            'stroke="#ddd" />'
        )
        parts.append(
            f'<text x="{margin_left - 10}" y="{y + 4:.2f}" text-anchor="end" font-size="12">'
            f"{y_value:.1f}</text>"
        )

    parts.extend(
        [
            f'<text x="{margin_left + plot_width / 2:.2f}" y="{height - 12}" text-anchor="middle" '
            'font-size="13">repetitions</text>',
            f'<text x="18" y="{margin_top + plot_height / 2:.2f}" text-anchor="middle" '
            f'font-size="13" transform="rotate(-90 18 {margin_top + plot_height / 2:.2f})">'
            "estimated 1RM (kg)</text>",
            # Curves
            f'<polyline points="{polyline(brzycki_values)}" fill="none" stroke="#4e79a7" '
            'stroke-width="2" stroke-dasharray="6,4" />',
            f'<polyline points="{polyline(epley_values)}" fill="none" stroke="#e15759" '
            'stroke-width="2" stroke-dasharray="6,4" />',
            f'<polyline points="{polyline(mayhew_values)}" fill="none" stroke="#59a14f" '
            'stroke-width="2" stroke-dasharray="6,4" />',
            f'<polyline points="{polyline(wathan_values)}" fill="none" stroke="#b07aa1" '
            'stroke-width="2" stroke-dasharray="6,4" />',
            f'<polyline points="{polyline(landers_values)}" fill="none" stroke="#76b7b2" '
            'stroke-width="2" stroke-dasharray="6,4" />',
            f'<polyline points="{polyline(desgorces_values)}" fill="none" stroke="#f28e2b" '
            'stroke-width="2" stroke-dasharray="2,4" />',
            f'<polyline points="{polyline(current_values)}" fill="none" stroke="#222" '
            'stroke-width="3" />',
        ]
    )

    legend_box_h = 88
    legend_x = margin_left + 10
    legend_y = margin_top + 10 + 18
    parts.extend(
        [
            f'<rect x="{legend_x}" y="{legend_y - 18}" width="260" height="{legend_box_h}" '
            'fill="white" stroke="#ccc" />',
            # Header row: current piecewise formula
            f'<line x1="{legend_x + 10}" y1="{legend_y}" x2="{legend_x + 52}" y2="{legend_y}" '
            'stroke="#222" stroke-width="3" />',
            f'<text x="{legend_x + 58}" y="{legend_y + 4}" font-size="12">'
            "Hybrid (current)</text>",
            # Row 1: Brzycki (left) | Landers (right)
            f'<line x1="{legend_x + 10}" y1="{legend_y + 20}" x2="{legend_x + 40}" '
            f'y2="{legend_y + 20}" stroke="#4e79a7" stroke-width="2" stroke-dasharray="6,4" />',
            f'<text x="{legend_x + 46}" y="{legend_y + 24}" font-size="12">Brzycki</text>',
            f'<line x1="{legend_x + 130}" y1="{legend_y + 20}" x2="{legend_x + 160}" '
            f'y2="{legend_y + 20}" stroke="#76b7b2" stroke-width="2" stroke-dasharray="6,4" />',
            f'<text x="{legend_x + 166}" y="{legend_y + 24}" font-size="12">Landers</text>',
            # Row 2: Desgorces (left) | Mayhew (right)
            f'<line x1="{legend_x + 10}" y1="{legend_y + 40}" x2="{legend_x + 40}" '
            f'y2="{legend_y + 40}" stroke="#f28e2b" stroke-width="2" stroke-dasharray="2,4" />',
            f'<text x="{legend_x + 46}" y="{legend_y + 44}" font-size="12">Desgorces</text>',
            f'<line x1="{legend_x + 130}" y1="{legend_y + 40}" x2="{legend_x + 160}" '
            f'y2="{legend_y + 40}" stroke="#59a14f" stroke-width="2" stroke-dasharray="6,4" />',
            f'<text x="{legend_x + 166}" y="{legend_y + 44}" font-size="12">Mayhew</text>',
            # Row 3: Epley (left) | Wathan (right)
            f'<line x1="{legend_x + 10}" y1="{legend_y + 60}" x2="{legend_x + 40}" '
            f'y2="{legend_y + 60}" stroke="#e15759" stroke-width="2" stroke-dasharray="6,4" />',
            f'<text x="{legend_x + 46}" y="{legend_y + 64}" font-size="12">Epley</text>',
            f'<line x1="{legend_x + 130}" y1="{legend_y + 60}" x2="{legend_x + 160}" '
            f'y2="{legend_y + 60}" stroke="#b07aa1" stroke-width="2" stroke-dasharray="6,4" />',
            f'<text x="{legend_x + 166}" y="{legend_y + 64}" font-size="12">Wathan</text>',
        ]
    )

    parts.append("</svg>")
    return "".join(parts)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--weight",
        type=float,
        default=100.0,
        help="Weight in kg used to visualize 1RM estimation (default: 100).",
    )
    parser.add_argument(
        "--max-reps",
        type=int,
        default=20,
        help="Maximum repetitions shown on x-axis (default: 20).",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("build/1rm_formula.svg"),
        help="Output SVG file path (default: build/1rm_formula.svg).",
    )
    args = parser.parse_args()

    if args.max_reps < 2:
        raise SystemExit("--max-reps must be at least 2")

    svg = build_svg(weight=args.weight, reps_max=args.max_reps)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(svg, encoding="utf-8")
    print(f"Created {args.output}")  # noqa: T201


if __name__ == "__main__":
    main()
