from __future__ import annotations

import datetime
import enum
from typing import Optional

from sqlalchemy import (
    Boolean,
    CheckConstraint,
    Date,
    Enum,
    Float,
    ForeignKey,
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
            "typeof(tigh) = 'integer' or typeof(tigh) = 'null'",
            name="tigh_type_integer_or_null",
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
        CheckConstraint(column("tigh") > 0, name="tigh_gt_0"),
        CheckConstraint(column("tricep") > 0, name="tricep_gt_0"),
        CheckConstraint(column("subscapular") > 0, name="subscapular_gt_0"),
        CheckConstraint(column("suprailiac") > 0, name="suprailiac_gt_0"),
        CheckConstraint(column("midaxillary") > 0, name="midaxillary_gt_0"),
    )

    user_id: Mapped[int] = mapped_column(
        ForeignKey("user.id", ondelete="CASCADE"), primary_key=True
    )
    date: Mapped[datetime.date] = mapped_column(Date, primary_key=True)
    chest: Mapped[Optional[int]] = mapped_column(Integer)
    abdominal: Mapped[Optional[int]] = mapped_column(Integer)
    tigh: Mapped[Optional[int]] = mapped_column(Integer)
    tricep: Mapped[Optional[int]] = mapped_column(Integer)
    subscapular: Mapped[Optional[int]] = mapped_column(Integer)
    suprailiac: Mapped[Optional[int]] = mapped_column(Integer)
    midaxillary: Mapped[Optional[int]] = mapped_column(Integer)


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

    sets: Mapped[list[WorkoutSet]] = relationship(
        "WorkoutSet", back_populates="exercise", cascade="all, delete-orphan"
    )
    routine_activities: Mapped[list[RoutineActivity]] = relationship(
        "RoutineActivity", back_populates="exercise", cascade="all, delete-orphan"
    )


class Routine(Base):
    __tablename__ = "routine"
    __table_args__ = (UniqueConstraint("user_id", "name"),)

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    user_id: Mapped[int] = mapped_column(ForeignKey("user.id", ondelete="CASCADE"), nullable=False)
    name: Mapped[str] = mapped_column(String, nullable=False)
    notes: Mapped[Optional[str]] = mapped_column(String)

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
    routine_section_id: Mapped[Optional[int]] = mapped_column(
        ForeignKey("routine_section.id", ondelete="CASCADE")
    )
    position: Mapped[int] = mapped_column(Integer, nullable=False)

    section: Mapped[RoutineSection] = relationship(
        "RoutineSection",
        back_populates="parts",
        foreign_keys=[routine_section_id],
    )

    __mapper_args__ = {
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
    routine_id: Mapped[Optional[int]] = mapped_column(ForeignKey("routine.id", ondelete="CASCADE"))
    rounds: Mapped[int] = mapped_column(Integer, nullable=False)

    parts: Mapped[list[RoutinePart]] = relationship(
        "RoutinePart",
        back_populates="section",
        foreign_keys=RoutinePart.routine_section_id,
    )
    routine: Mapped[Routine] = relationship("Routine", back_populates="sections")

    __mapper_args__ = {
        "polymorphic_identity": "routine_section",
        "inherit_condition": id == RoutinePart.id,
    }


class RoutineActivity(RoutinePart):
    __tablename__ = "routine_activity"
    __table_args__ = (
        CheckConstraint("typeof(duration) = 'integer'", name="duration_type_integer"),
        CheckConstraint(column("duration") >= 0, name="duration_ge_0"),
        CheckConstraint("typeof(tempo) = 'integer'", name="tempo_type_integer"),
        CheckConstraint(column("tempo") >= 0, name="tempo_ge_0"),
    )

    id: Mapped[int] = mapped_column(Integer, ForeignKey("routine_part.id"), primary_key=True)
    exercise_id: Mapped[Optional[int]] = mapped_column(
        ForeignKey("exercise.id", ondelete="CASCADE")
    )
    duration: Mapped[int] = mapped_column(Integer, nullable=False)
    tempo: Mapped[int] = mapped_column(Integer, nullable=False)
    automatic: Mapped[bool] = mapped_column(Boolean, nullable=False)

    exercise: Mapped[Exercise] = relationship("Exercise", back_populates="routine_activities")

    __mapper_args__ = {
        "polymorphic_identity": "routine_activity",
        "inherit_condition": id == RoutinePart.id,
    }


class Workout(Base):
    __tablename__ = "workout"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    user_id: Mapped[int] = mapped_column(ForeignKey("user.id", ondelete="CASCADE"), nullable=False)
    routine_id: Mapped[Optional[int]] = mapped_column(ForeignKey("routine.id", ondelete="CASCADE"))
    date: Mapped[datetime.date] = mapped_column(Date, nullable=False)
    notes: Mapped[Optional[str]] = mapped_column(String)

    routine: Mapped[Routine] = relationship("Routine", back_populates="workouts")
    sets: Mapped[list[WorkoutSet]] = relationship(
        "WorkoutSet", back_populates="workout", cascade="all, delete-orphan"
    )


class WorkoutSet(Base):
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
        CheckConstraint(column("position") > 0, name="position_gt_0"),
        CheckConstraint(column("reps") > 0, name="reps_gt_0"),
        CheckConstraint(column("time") > 0, name="time_gt_0"),
        CheckConstraint(column("weight") > 0, name="weight_gt_0"),
        CheckConstraint(column("rpe") >= 0, name="rpe_ge_0"),
        CheckConstraint(column("rpe") <= 10, name="rpe_le_10"),
    )

    workout_id: Mapped[int] = mapped_column(
        ForeignKey("workout.id", ondelete="CASCADE"), primary_key=True
    )
    position: Mapped[int] = mapped_column(Integer, primary_key=True)
    exercise_id: Mapped[int] = mapped_column(
        ForeignKey("exercise.id", ondelete="CASCADE"), nullable=False
    )
    reps: Mapped[Optional[int]] = mapped_column(Integer)
    time: Mapped[Optional[int]] = mapped_column(Integer)
    weight: Mapped[Optional[float]] = mapped_column(Float)
    rpe: Mapped[Optional[float]] = mapped_column(Float)

    workout: Mapped[Workout] = relationship("Workout", back_populates="sets")
    exercise: Mapped[Exercise] = relationship("Exercise", back_populates="sets")
