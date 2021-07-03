from __future__ import annotations

from datetime import date
from enum import IntEnum
from typing import Optional

from sqlalchemy import (
    CheckConstraint,
    Column,
    Date,
    Enum,
    Float,
    ForeignKey,
    Integer,
    String,
    UniqueConstraint,
    column,
)
from sqlalchemy.orm import relationship

from valens.database import Base


class Sex(IntEnum):
    FEMALE = 0
    MALE = 1


class User(Base):
    __tablename__ = "user"

    id: int = Column(Integer, primary_key=True)
    name: str = Column(String, unique=True, nullable=False)
    sex: Sex = Column(Enum(Sex), nullable=False)

    body_weight: list[BodyWeight] = relationship(
        "BodyWeight", backref="user", cascade="all, delete-orphan"
    )
    body_fat: list[BodyFat] = relationship("BodyFat", backref="user", cascade="all, delete-orphan")
    period: list[Period] = relationship("Period", backref="user", cascade="all, delete-orphan")
    exercises: list[Exercise] = relationship(
        "Exercise", backref="user", cascade="all, delete-orphan"
    )
    routines: list[Routine] = relationship("Routine", backref="user", cascade="all, delete-orphan")
    workouts: list[Workout] = relationship("Workout", backref="user", cascade="all, delete-orphan")


class BodyWeight(Base):
    __tablename__ = "body_weight"

    user_id: int = Column(ForeignKey("user.id"), primary_key=True)
    date: date = Column(Date, primary_key=True)
    weight: float = Column(Float, CheckConstraint(column("weight") > 0), nullable=False)


class BodyFat(Base):
    __tablename__ = "body_fat"

    user_id: int = Column(ForeignKey("user.id"), primary_key=True)
    date: date = Column(Date, primary_key=True)
    chest: Optional[int] = Column(Integer, CheckConstraint(column("chest") > 0))
    abdominal: Optional[int] = Column(Integer, CheckConstraint(column("abdominal") > 0))
    tigh: Optional[int] = Column(Integer, CheckConstraint(column("tigh") > 0))
    tricep: Optional[int] = Column(Integer, CheckConstraint(column("tricep") > 0))
    subscapular: Optional[int] = Column(Integer, CheckConstraint(column("subscapular") > 0))
    suprailiac: Optional[int] = Column(Integer, CheckConstraint(column("suprailiac") > 0))
    midaxillary: Optional[int] = Column(Integer, CheckConstraint(column("midaxillary") > 0))


class Period(Base):
    __tablename__ = "period"

    user_id: int = Column(ForeignKey("user.id"), primary_key=True)
    date: date = Column(Date, primary_key=True)
    intensity: int = Column(
        Integer,
        CheckConstraint(column("intensity") >= 1),
        CheckConstraint(column("intensity") <= 4),
        nullable=False,
    )


class Exercise(Base):
    __tablename__ = "exercise"
    __table_args__ = (UniqueConstraint("user_id", "name"),)

    id: int = Column(Integer, primary_key=True)
    user_id: int = Column(ForeignKey("user.id"), nullable=False)
    name: str = Column(String, nullable=False)

    sets: list[WorkoutSet] = relationship(
        "WorkoutSet", back_populates="exercise", cascade="all, delete-orphan"
    )


class Routine(Base):
    __tablename__ = "routine"
    __table_args__ = (UniqueConstraint("user_id", "name"),)

    id: int = Column(Integer, primary_key=True)
    user_id: int = Column(ForeignKey("user.id"), nullable=False)
    name: str = Column(String, nullable=False)
    notes: str = Column(String)

    exercises: list[RoutineExercise] = relationship(
        "RoutineExercise", backref="routine", cascade="all, delete-orphan"
    )
    workouts: list[Workout] = relationship("Workout", back_populates="routine")


class RoutineExercise(Base):
    __tablename__ = "routine_exercise"

    routine_id: int = Column(ForeignKey("routine.id"), primary_key=True)
    position: int = Column(Integer, CheckConstraint(column("position") > 0), primary_key=True)
    exercise_id: int = Column(ForeignKey("exercise.id"), nullable=False)
    sets: int = Column(Integer, CheckConstraint(column("sets") > 0), nullable=False)

    exercise: Exercise = relationship("Exercise")


class Workout(Base):
    __tablename__ = "workout"

    id: int = Column(Integer, primary_key=True)
    user_id: int = Column(ForeignKey("user.id"), nullable=False)
    routine_id: int = Column(ForeignKey("routine.id"))
    date: date = Column(Date, nullable=False)
    notes: str = Column(String)

    routine: Routine = relationship("Routine", back_populates="workouts")
    sets: list[WorkoutSet] = relationship(
        "WorkoutSet", back_populates="workout", cascade="all, delete-orphan"
    )


class WorkoutSet(Base):
    __tablename__ = "workout_set"

    workout_id: int = Column(ForeignKey("workout.id"), primary_key=True)
    position: int = Column(Integer, CheckConstraint(column("position") > 0), primary_key=True)
    exercise_id: int = Column(ForeignKey("exercise.id"), nullable=False)
    reps: int = Column(Integer, CheckConstraint(column("reps") > 0))
    time: int = Column(Integer, CheckConstraint(column("time") > 0))
    weight: float = Column(Float, CheckConstraint(column("weight") > 0))
    rpe: float = Column(
        Float, CheckConstraint(column("rpe") >= 0), CheckConstraint(column("rpe") <= 10)
    )

    workout: Workout = relationship("Workout", back_populates="sets")
    exercise: Exercise = relationship("Exercise", back_populates="sets")
