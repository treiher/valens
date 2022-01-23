CREATE TABLE IF NOT EXISTS "alembic_version" (
	version_num VARCHAR(32) NOT NULL,
	CONSTRAINT alembic_version_pkc PRIMARY KEY (version_num)
);
CREATE TABLE IF NOT EXISTS "user" (
	id INTEGER NOT NULL,
	name VARCHAR NOT NULL,
	sex VARCHAR(6) NOT NULL,
	CONSTRAINT pk_user PRIMARY KEY (id),
	CONSTRAINT uq_user_name UNIQUE (name)
);
CREATE TABLE IF NOT EXISTS "body_weight" (
	user_id INTEGER NOT NULL,
	date DATE NOT NULL,
	weight FLOAT NOT NULL,
	CONSTRAINT pk_body_weight PRIMARY KEY (user_id, date),
	CONSTRAINT ck_body_weight_weight_gt_0 CHECK (weight > 0),
	CONSTRAINT ck_body_weight_weight_gt_0 CHECK (weight > 0),
	CONSTRAINT fk_body_weight_user_id_user FOREIGN KEY(user_id) REFERENCES user (id)
);
CREATE TABLE IF NOT EXISTS "body_fat" (
	user_id INTEGER NOT NULL,
	date DATE NOT NULL,
	chest INTEGER,
	abdominal INTEGER,
	tigh INTEGER,
	tricep INTEGER,
	subscapular INTEGER,
	suprailiac INTEGER,
	midaxillary INTEGER,
	CONSTRAINT pk_body_fat PRIMARY KEY (user_id, date),
	CONSTRAINT ck_body_fat_chest_gt_0 CHECK (chest > 0),
	CONSTRAINT ck_body_fat_abdominal_gt_0 CHECK (abdominal > 0),
	CONSTRAINT ck_body_fat_tigh_gt_0 CHECK (tigh > 0),
	CONSTRAINT ck_body_fat_tricep_gt_0 CHECK (tricep > 0),
	CONSTRAINT ck_body_fat_subscapular_gt_0 CHECK (subscapular > 0),
	CONSTRAINT ck_body_fat_suprailiac_gt_0 CHECK (suprailiac > 0),
	CONSTRAINT ck_body_fat_midaxillary_gt_0 CHECK (midaxillary > 0),
	CONSTRAINT ck_body_fat_subscapular_gt_0 CHECK (subscapular > 0),
	CONSTRAINT ck_body_fat_chest_gt_0 CHECK (chest > 0),
	CONSTRAINT ck_body_fat_suprailiac_gt_0 CHECK (suprailiac > 0),
	CONSTRAINT ck_body_fat_tigh_gt_0 CHECK (tigh > 0),
	CONSTRAINT ck_body_fat_tricep_gt_0 CHECK (tricep > 0),
	CONSTRAINT ck_body_fat_abdominal_gt_0 CHECK (abdominal > 0),
	CONSTRAINT fk_body_fat_user_id_user FOREIGN KEY(user_id) REFERENCES user (id)
);
CREATE TABLE IF NOT EXISTS "period" (
	user_id INTEGER NOT NULL,
	date DATE NOT NULL,
	intensity INTEGER NOT NULL,
	CONSTRAINT pk_period PRIMARY KEY (user_id, date),
	CONSTRAINT ck_period_intensity_ge_1 CHECK (intensity >= 1),
	CONSTRAINT ck_period_intensity_le_4 CHECK (intensity <= 4),
	CONSTRAINT ck_period_intensity_le_4 CHECK (intensity <= 4),
	CONSTRAINT ck_period_intensity_ge_1 CHECK (intensity >= 1),
	CONSTRAINT fk_period_user_id_user FOREIGN KEY(user_id) REFERENCES user (id)
);
CREATE TABLE IF NOT EXISTS "routine_exercise" (
	routine_id INTEGER NOT NULL,
	position INTEGER NOT NULL,
	exercise_id INTEGER NOT NULL,
	sets INTEGER NOT NULL,
	CONSTRAINT pk_routine_exercise PRIMARY KEY (routine_id, position),
	CONSTRAINT ck_routine_exercise_position_gt_0 CHECK (position > 0),
	CONSTRAINT ck_routine_exercise_sets_gt_0 CHECK (sets > 0),
	CONSTRAINT ck_routine_exercise_position_gt_0 CHECK (position > 0),
	CONSTRAINT fk_routine_exercise_routine_id_routine FOREIGN KEY(routine_id) REFERENCES routine (id),
	CONSTRAINT ck_routine_exercise_sets_gt_0 CHECK (sets > 0),
	CONSTRAINT fk_routine_exercise_exercise_id_exercise FOREIGN KEY(exercise_id) REFERENCES exercise (id)
);
CREATE TABLE IF NOT EXISTS "workout_set" (
	workout_id INTEGER NOT NULL,
	position INTEGER NOT NULL,
	exercise_id INTEGER NOT NULL,
	reps INTEGER,
	time INTEGER,
	weight FLOAT,
	rpe FLOAT,
	CONSTRAINT pk_workout_set PRIMARY KEY (workout_id, position),
	CONSTRAINT ck_workout_set_position_gt_0 CHECK (position > 0),
	CONSTRAINT ck_workout_set_reps_gt_0 CHECK (reps > 0),
	CONSTRAINT ck_workout_set_time_gt_0 CHECK (time > 0),
	CONSTRAINT ck_workout_set_weight_gt_0 CHECK (weight > 0),
	CONSTRAINT ck_workout_set_rpe_ge_0 CHECK (rpe >= 0),
	CONSTRAINT ck_workout_set_rpe_le_10 CHECK (rpe <= 10),
	CONSTRAINT ck_workout_set_position_gt_0 CHECK (position > 0),
	CONSTRAINT fk_workout_set_workout_id_workout FOREIGN KEY(workout_id) REFERENCES workout (id),
	CONSTRAINT ck_workout_set_weight_gt_0 CHECK (weight > 0),
	CONSTRAINT ck_workout_set_rpe_ge_0 CHECK (rpe >= 0),
	CONSTRAINT ck_workout_set_reps_gt_0 CHECK (reps > 0),
	CONSTRAINT ck_workout_set_time_gt_0 CHECK (time > 0),
	CONSTRAINT ck_workout_set_rpe_le_10 CHECK (rpe <= 10),
	CONSTRAINT fk_workout_set_exercise_id_exercise FOREIGN KEY(exercise_id) REFERENCES exercise (id)
);
CREATE TABLE IF NOT EXISTS "exercise" (
	id INTEGER NOT NULL,
	user_id INTEGER NOT NULL,
	name VARCHAR NOT NULL,
	CONSTRAINT pk_exercise PRIMARY KEY (id),
	CONSTRAINT uq_exercise_user_id UNIQUE (user_id, name),
	CONSTRAINT fk_exercise_user_id_user FOREIGN KEY(user_id) REFERENCES user (id)
);
CREATE TABLE IF NOT EXISTS "routine" (
	id INTEGER NOT NULL,
	user_id INTEGER NOT NULL,
	name VARCHAR NOT NULL,
	notes VARCHAR,
	CONSTRAINT pk_routine PRIMARY KEY (id),
	CONSTRAINT uq_routine_user_id UNIQUE (user_id, name),
	CONSTRAINT fk_routine_user_id_user FOREIGN KEY(user_id) REFERENCES user (id)
);
CREATE TABLE IF NOT EXISTS "workout" (
	id INTEGER NOT NULL,
	user_id INTEGER NOT NULL,
	date DATE NOT NULL,
	notes VARCHAR,
	CONSTRAINT pk_workout PRIMARY KEY (id),
	CONSTRAINT fk_workout_user_id_user FOREIGN KEY(user_id) REFERENCES user (id)
);
