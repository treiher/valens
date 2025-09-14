from __future__ import annotations

import datetime
import enum

from sqlalchemy import (
    CheckConstraint,
    Constraint,
    Date,
    Enum,
    Float,
    ForeignKey,
    ForeignKeyConstraint,
    Integer,
    MetaData,
    String,
    UniqueConstraint,
    column,
)
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship

# Alembic's autogenerate does not support CHECK constraints
# (https://github.com/sqlalchemy/alembic/issues/508). CHECK constraints get lost when running
# "batch" migrations for SQLite and must be stated explicitly in the migration script
# (https://alembic.sqlalchemy.org/en/latest/batch.html#including-check-constraints).
# CHECK constraints are only specified at the table level to enable the schema to be kept identical
# when applying migrations despite the aforementioned limitations.


class Base(DeclarativeBase):
    metadata = MetaData(
        naming_convention={
            "ix": "ix_%(column_0_label)s",
            "uq": "uq_%(table_name)s_%(column_0_name)s",
            "ck": "ck_%(table_name)s_%(constraint_name)s",
            "fk": "fk_%(table_name)s_%(column_0_name)s_%(referred_table_name)s",
            "pk": "pk_%(table_name)s",
        }
    )


class Sex(enum.IntEnum):
    FEMALE = 0
    MALE = 1


class User(Base):
    __tablename__ = "user"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    name: Mapped[str] = mapped_column(String, unique=True, nullable=False)
    sex: Mapped[Sex] = mapped_column(Enum(Sex), nullable=False)

    body_weight: Mapped[list[BodyWeight]] = relationship(
        "BodyWeight", backref="user", cascade="all, delete-orphan", passive_deletes=True
    )
    body_fat: Mapped[list[BodyFat]] = relationship(
        "BodyFat", backref="user", cascade="all, delete-orphan", passive_deletes=True
    )
    period: Mapped[list[Period]] = relationship(
        "Period", backref="user", cascade="all, delete-orphan", passive_deletes=True
    )
    exercises: Mapped[list[Exercise]] = relationship(
        "Exercise", backref="user", cascade="all, delete-orphan", passive_deletes=True
    )
    routines: Mapped[list[Routine]] = relationship(
        "Routine", backref="user", cascade="all, delete-orphan", passive_deletes=True
    )
    workouts: Mapped[list[Workout]] = relationship(
        "Workout", backref="user", cascade="all, delete-orphan", passive_deletes=True
    )


class BodyWeight(Base):
    __tablename__ = "body_weight"
    __table_args__ = (
        CheckConstraint("typeof(weight) = 'real'", name="weight_type_real"),
        CheckConstraint(column("weight") > 0, name="weight_gt_0"),
    )

    user_id: Mapped[int] = mapped_column(
        ForeignKey("user.id", ondelete="CASCADE"), primary_key=True
    )
    date: Mapped[datetime.date] = mapped_column(Date, primary_key=True)
    weight: Mapped[float] = mapped_column(Float, nullable=False)


class BodyFat(Base):
    __tablename__ = "body_fat"
    __table_args__ = (
        CheckConstraint(
            "typeof(chest) = 'integer' or typeof(chest) = 'null'",
            name="chest_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(abdominal) = 'integer' or typeof(abdominal) = 'null'",
            name="abdominal_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(thigh) = 'integer' or typeof(thigh) = 'null'",
            name="thigh_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(tricep) = 'integer' or typeof(tricep) = 'null'",
            name="tricep_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(subscapular) = 'integer' or typeof(subscapular) = 'null'",
            name="subscapular_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(suprailiac) = 'integer' or typeof(suprailiac) = 'null'",
            name="suprailiac_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(midaxillary) = 'integer' or typeof(midaxillary) = 'null'",
            name="midaxillary_type_integer_or_null",
        ),
        CheckConstraint(column("chest") > 0, name="chest_gt_0"),
        CheckConstraint(column("abdominal") > 0, name="abdominal_gt_0"),
        CheckConstraint(column("thigh") > 0, name="thigh_gt_0"),
        CheckConstraint(column("tricep") > 0, name="tricep_gt_0"),
        CheckConstraint(column("subscapular") > 0, name="subscapular_gt_0"),
        CheckConstraint(column("suprailiac") > 0, name="suprailiac_gt_0"),
        CheckConstraint(column("midaxillary") > 0, name="midaxillary_gt_0"),
    )

    user_id: Mapped[int] = mapped_column(
        ForeignKey("user.id", ondelete="CASCADE"), primary_key=True
    )
    date: Mapped[datetime.date] = mapped_column(Date, primary_key=True)
    chest: Mapped[int | None] = mapped_column(Integer)
    abdominal: Mapped[int | None] = mapped_column(Integer)
    thigh: Mapped[int | None] = mapped_column(Integer)
    tricep: Mapped[int | None] = mapped_column(Integer)
    subscapular: Mapped[int | None] = mapped_column(Integer)
    suprailiac: Mapped[int | None] = mapped_column(Integer)
    midaxillary: Mapped[int | None] = mapped_column(Integer)


class Period(Base):
    __tablename__ = "period"
    __table_args__ = (
        CheckConstraint("typeof(intensity) = 'integer'", name="intensity_type_integer"),
        CheckConstraint(column("intensity") >= 1, name="intensity_ge_1"),
        CheckConstraint(column("intensity") <= 4, name="intensity_le_4"),
    )

    user_id: Mapped[int] = mapped_column(
        ForeignKey("user.id", ondelete="CASCADE"), primary_key=True
    )
    date: Mapped[datetime.date] = mapped_column(Date, primary_key=True)
    intensity: Mapped[int] = mapped_column(Integer, nullable=False)


class Exercise(Base):
    __tablename__ = "exercise"
    __table_args__ = (UniqueConstraint("user_id", "name"),)

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    user_id: Mapped[int] = mapped_column(ForeignKey("user.id", ondelete="CASCADE"), nullable=False)
    name: Mapped[str] = mapped_column(String, nullable=False)

    muscles: Mapped[list[ExerciseMuscle]] = relationship(
        "ExerciseMuscle", backref="exercise", cascade="all, delete-orphan"
    )
    sets: Mapped[list[WorkoutSet]] = relationship(
        "WorkoutSet", back_populates="exercise", cascade="all, delete-orphan"
    )
    routine_activities: Mapped[list[RoutineActivity]] = relationship(
        "RoutineActivity", back_populates="exercise", cascade="all, delete-orphan"
    )


class ExerciseMuscle(Base):
    __tablename__ = "exercise_muscle"
    __table_args__ = (
        UniqueConstraint("user_id", "exercise_id", "muscle_id"),
        CheckConstraint("typeof(muscle_id) = 'integer'", name="muscle_id_integer"),
        CheckConstraint("typeof(stimulus) = 'integer'", name="stimulus_integer"),
        CheckConstraint(column("stimulus") >= 1, name="stimulus_ge_1"),
        CheckConstraint(column("stimulus") <= 100, name="stimulus_le_100"),
    )

    user_id: Mapped[int] = mapped_column(
        ForeignKey("user.id", ondelete="CASCADE"), nullable=False, primary_key=True
    )
    exercise_id: Mapped[int] = mapped_column(
        ForeignKey("exercise.id", ondelete="CASCADE"), nullable=False, primary_key=True
    )
    muscle_id: Mapped[int] = mapped_column(Integer, nullable=False, primary_key=True)
    stimulus: Mapped[int] = mapped_column(Integer, nullable=False)


class Routine(Base):
    __tablename__ = "routine"
    __table_args__ = (UniqueConstraint("user_id", "name"),)

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    user_id: Mapped[int] = mapped_column(ForeignKey("user.id", ondelete="CASCADE"), nullable=False)
    name: Mapped[str] = mapped_column(String, nullable=False)
    notes: Mapped[str | None] = mapped_column(String)
    archived: Mapped[bool] = mapped_column(default=False)

    sections: Mapped[list[RoutineSection]] = relationship(
        "RoutineSection", back_populates="routine", cascade="all, delete-orphan"
    )
    workouts: Mapped[list[Workout]] = relationship("Workout", back_populates="routine")


class RoutinePart(Base):
    __tablename__ = "routine_part"
    __table_args__: tuple[CheckConstraint, ...] = (
        CheckConstraint("typeof(position) = 'integer'", name="position_type_integer"),
        CheckConstraint("typeof(type) = 'text'", name="type_type_text"),
        CheckConstraint(column("position") > 0, name="position_gt_0"),
    )

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    type: Mapped[str] = mapped_column(String, nullable=False)
    routine_section_id: Mapped[int | None] = mapped_column(
        ForeignKey("routine_section.id", ondelete="CASCADE")
    )
    position: Mapped[int] = mapped_column(Integer, nullable=False)

    section: Mapped[RoutineSection] = relationship(
        "RoutineSection",
        back_populates="parts",
        foreign_keys=[routine_section_id],
    )

    __mapper_args__ = {  # noqa: RUF012
        "polymorphic_identity": "routine_part",
        "polymorphic_on": type,
    }


class RoutineSection(RoutinePart):
    __tablename__ = "routine_section"
    __table_args__ = (
        CheckConstraint("typeof(rounds) = 'integer'", name="rounds_type_integer"),
        CheckConstraint(column("rounds") > 0, name="rounds_gt_0"),
    )

    id: Mapped[int] = mapped_column(Integer, ForeignKey("routine_part.id"), primary_key=True)
    routine_id: Mapped[int | None] = mapped_column(ForeignKey("routine.id", ondelete="CASCADE"))
    rounds: Mapped[int] = mapped_column(Integer, nullable=False)

    parts: Mapped[list[RoutinePart]] = relationship(
        "RoutinePart",
        back_populates="section",
        foreign_keys=RoutinePart.routine_section_id,
    )
    routine: Mapped[Routine] = relationship("Routine", back_populates="sections")

    __mapper_args__ = {  # noqa: RUF012
        "polymorphic_identity": "routine_section",
        "polymorphic_load": "selectin",
        "inherit_condition": id == RoutinePart.id,
    }


class RoutineActivity(RoutinePart):
    __tablename__ = "routine_activity"
    __table_args__ = (
        CheckConstraint("typeof(reps) = 'integer'", name="reps_type_integer"),
        CheckConstraint(column("reps") >= 0, name="reps_ge_0"),
        CheckConstraint("typeof(time) = 'integer'", name="time_type_integer"),
        CheckConstraint(column("time") >= 0, name="time_ge_0"),
        CheckConstraint("typeof(weight) = 'real'", name="weight_type_real"),
        CheckConstraint(column("weight") >= 0, name="weight_ge_0"),
        CheckConstraint("typeof(rpe) = 'real'", name="rpe_type_real"),
        CheckConstraint(column("rpe") >= 0, name="rpe_ge_0"),
        CheckConstraint(column("rpe") <= 10, name="rpe_le_10"),
    )

    id: Mapped[int] = mapped_column(Integer, ForeignKey("routine_part.id"), primary_key=True)
    exercise_id: Mapped[int | None] = mapped_column(ForeignKey("exercise.id", ondelete="CASCADE"))
    reps: Mapped[int]
    time: Mapped[int]
    weight: Mapped[float]
    rpe: Mapped[float]
    automatic: Mapped[bool]

    exercise: Mapped[Exercise] = relationship("Exercise", back_populates="routine_activities")

    __mapper_args__ = {  # noqa: RUF012
        "polymorphic_identity": "routine_activity",
        "polymorphic_load": "selectin",
        "inherit_condition": id == RoutinePart.id,
    }


class Workout(Base):
    __tablename__ = "workout"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    user_id: Mapped[int] = mapped_column(ForeignKey("user.id", ondelete="CASCADE"), nullable=False)
    routine_id: Mapped[int | None] = mapped_column(ForeignKey("routine.id", ondelete="CASCADE"))
    date: Mapped[datetime.date] = mapped_column(Date, nullable=False)
    notes: Mapped[str | None] = mapped_column(String)

    routine: Mapped[Routine] = relationship("Routine", back_populates="workouts")
    elements: Mapped[list[WorkoutElement]] = relationship(
        "WorkoutElement", back_populates="workout", cascade="all, delete-orphan"
    )


class WorkoutElement(Base):
    __tablename__ = "workout_element"
    __table_args__: tuple[Constraint, ...] = (
        CheckConstraint(
            "typeof(position) = 'integer'",
            name="position_type_integer",
        ),
        CheckConstraint("typeof(type) = 'text'", name="type_type_text"),
        CheckConstraint(
            "typeof(automatic) = 'integer'",
            name="automatic_type_integer",
        ),
        CheckConstraint(column("position") > 0, name="position_gt_0"),
        CheckConstraint(column("automatic") >= 0, name="rpe_ge_0"),
        CheckConstraint(column("automatic") <= 1, name="rpe_le_1"),
    )

    workout_id: Mapped[int] = mapped_column(
        ForeignKey("workout.id", ondelete="CASCADE"), primary_key=True
    )
    position: Mapped[int] = mapped_column(primary_key=True)
    type: Mapped[str]
    automatic: Mapped[bool] = mapped_column(default=False)

    workout: Mapped[Workout] = relationship("Workout", back_populates="elements")

    __mapper_args__ = {  # noqa: RUF012
        "polymorphic_identity": "activity",
        "polymorphic_on": "type",
    }


class WorkoutSet(WorkoutElement):
    __tablename__ = "workout_set"
    __table_args__ = (
        CheckConstraint(
            "typeof(position) = 'integer'",
            name="position_type_integer",
        ),
        CheckConstraint(
            "typeof(reps) = 'integer' or typeof(reps) = 'null'",
            name="reps_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(time) = 'integer' or typeof(time) = 'null'",
            name="time_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(weight) = 'real' or typeof(weight) = 'null'",
            name="weight_type_real_or_null",
        ),
        CheckConstraint(
            "typeof(rpe) = 'real' or typeof(rpe) = 'null'",
            name="rpe_type_real_or_null",
        ),
        CheckConstraint(
            "typeof(target_reps) = 'integer' or typeof(target_reps) = 'null'",
            name="target_reps_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(target_time) = 'integer' or typeof(target_time) = 'null'",
            name="target_time_type_integer_or_null",
        ),
        CheckConstraint(
            "typeof(target_weight) = 'real' or typeof(target_weight) = 'null'",
            name="target_weight_type_real_or_null",
        ),
        CheckConstraint(
            "typeof(target_rpe) = 'real' or typeof(target_rpe) = 'null'",
            name="target_rpe_type_real_or_null",
        ),
        CheckConstraint(column("position") > 0, name="position_gt_0"),
        CheckConstraint(column("reps") > 0, name="reps_gt_0"),
        CheckConstraint(column("time") > 0, name="time_gt_0"),
        CheckConstraint(column("weight") > 0, name="weight_gt_0"),
        CheckConstraint(column("rpe") >= 0, name="rpe_ge_0"),
        CheckConstraint(column("rpe") <= 10, name="rpe_le_10"),
        CheckConstraint(column("target_reps") > 0, name="target_reps_gt_0"),
        CheckConstraint(column("target_time") > 0, name="target_time_gt_0"),
        CheckConstraint(column("target_weight") > 0, name="target_weight_gt_0"),
        CheckConstraint(column("target_rpe") >= 0, name="target_rpe_ge_0"),
        CheckConstraint(column("target_rpe") <= 10, name="target_rpe_le_10"),
        ForeignKeyConstraint(
            ["workout_id", "position"],
            [WorkoutElement.workout_id, WorkoutElement.position],
            ondelete="CASCADE",
        ),
    )

    workout_id: Mapped[int] = mapped_column(primary_key=True)
    position: Mapped[int] = mapped_column(primary_key=True)
    exercise_id: Mapped[int] = mapped_column(
        ForeignKey("exercise.id", ondelete="CASCADE"), nullable=False
    )
    reps: Mapped[int | None]
    time: Mapped[int | None]
    weight: Mapped[float | None]
    rpe: Mapped[float | None]
    target_reps: Mapped[int | None]
    target_time: Mapped[int | None]
    target_weight: Mapped[float | None]
    target_rpe: Mapped[float | None]

    exercise: Mapped[Exercise] = relationship("Exercise", back_populates="sets")

    __mapper_args__ = {  # noqa: RUF012
        "polymorphic_identity": "set",
        "polymorphic_load": "selectin",
    }


class WorkoutRest(WorkoutElement):
    __tablename__ = "workout_rest"
    __table_args__ = (
        CheckConstraint(
            "typeof(position) = 'integer'",
            name="position_type_integer",
        ),
        CheckConstraint(
            "typeof(target_time) = 'integer' or typeof(target_time) = 'null'",
            name="target_time_type_integer_or_null",
        ),
        CheckConstraint(column("position") > 0, name="position_gt_0"),
        CheckConstraint(column("target_time") > 0, name="target_time_gt_0"),
        ForeignKeyConstraint(
            ["workout_id", "position"],
            [WorkoutElement.workout_id, WorkoutElement.position],
            ondelete="CASCADE",
        ),
    )

    workout_id: Mapped[int] = mapped_column(primary_key=True)
    position: Mapped[int] = mapped_column(primary_key=True)
    target_time: Mapped[int | None]

    __mapper_args__ = {  # noqa: RUF012
        "polymorphic_identity": "rest",
        "polymorphic_load": "selectin",
    }
