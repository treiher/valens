#!/usr/bin/env python

import argparse
import sys
from typing import Union

import matplotlib.pyplot as plt

from valens import diagram, storage, utils


def main() -> Union[int, str]:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="subcommand")

    parser_init = subparsers.add_parser("init", help="initialize data storage")
    parser_init.set_defaults(func=init)

    parser_show = subparsers.add_parser("show", help="show exercise")
    sp_show = parser_show.add_subparsers(dest="subcommand")
    sp_show_wo = sp_show.add_parser("wo", help="show workouts")
    sp_show_wo.set_defaults(func=show_workouts)
    sp_show_ex = sp_show.add_parser("ex", help="show exercise")
    sp_show_ex.add_argument("exercise", metavar="NAME", type=str, help="exercise")
    sp_show_ex.set_defaults(func=show_exercise)
    sp_show_bw = sp_show.add_parser("bw", help="show bodyweight")
    sp_show_bw.set_defaults(func=show_bodyweight)

    parser_list = subparsers.add_parser("list", help="list exercises")
    parser_list.add_argument(
        "--last", action="store_true", help="list only excercises of last workout"
    )
    parser_list.add_argument("--short", action="store_true", help="list only excercise names")
    parser_list.set_defaults(func=list_exercises)

    args = parser.parse_args(sys.argv[1:])

    if not args.subcommand:
        parser.print_usage()
        return 2

    args.func(args)

    return 0


def init(args: argparse.Namespace) -> None:  # pylint: disable=unused-argument
    storage.initialize()


def list_exercises(args: argparse.Namespace) -> None:
    df = storage.read_sets(1)

    if args.last:
        last_exercises = list(
            df.loc[lambda x: x["date"] == df["date"].iloc[-1]].groupby(["exercise"]).groups
        )
        df = df.loc[lambda x: x["exercise"].isin(last_exercises)]

    for exercise, log in df.groupby(["exercise"]):
        print(f"\n### {exercise}\n")
        for date, sets in log.groupby(["date"]):
            print(
                f"- {date}: "
                + "-".join(
                    utils.format_set(set_tuple[1:])
                    for set_tuple in sets.loc[:, ["reps", "time", "weight", "rpe"]].itertuples()
                )
            )


def show_workouts(args: argparse.Namespace) -> None:
    # pylint: disable=unused-argument
    diagram.plot_workouts(1)
    plt.show()


def show_exercise(args: argparse.Namespace) -> None:
    diagram.plot_exercise(1, args.exercise)
    plt.show()


def show_bodyweight(args: argparse.Namespace) -> None:
    # pylint: disable=unused-argument
    diagram.plot_bodyweight(1)
    plt.show()
