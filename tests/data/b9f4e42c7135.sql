BEGIN TRANSACTION;
CREATE TABLE alembic_version (
	version_num VARCHAR(32) NOT NULL,
	CONSTRAINT alembic_version_pkc PRIMARY KEY (version_num)
);
INSERT INTO "alembic_version" VALUES('b9f4e42c7135');
CREATE TABLE body_fat (
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
	CONSTRAINT ck_body_fat_chest_type_integer_or_null CHECK (typeof(chest) = 'integer' or typeof(chest) = 'null'),
	CONSTRAINT ck_body_fat_abdominal_type_integer_or_null CHECK (typeof(abdominal) = 'integer' or typeof(abdominal) = 'null'),
	CONSTRAINT ck_body_fat_tigh_type_integer_or_null CHECK (typeof(tigh) = 'integer' or typeof(tigh) = 'null'),
	CONSTRAINT ck_body_fat_tricep_type_integer_or_null CHECK (typeof(tricep) = 'integer' or typeof(tricep) = 'null'),
	CONSTRAINT ck_body_fat_subscapular_type_integer_or_null CHECK (typeof(subscapular) = 'integer' or typeof(subscapular) = 'null'),
	CONSTRAINT ck_body_fat_suprailiac_type_integer_or_null CHECK (typeof(suprailiac) = 'integer' or typeof(suprailiac) = 'null'),
	CONSTRAINT ck_body_fat_midaxillary_type_integer_or_null CHECK (typeof(midaxillary) = 'integer' or typeof(midaxillary) = 'null'),
	CONSTRAINT ck_body_fat_chest_gt_0 CHECK (chest > 0),
	CONSTRAINT ck_body_fat_abdominal_gt_0 CHECK (abdominal > 0),
	CONSTRAINT ck_body_fat_tigh_gt_0 CHECK (tigh > 0),
	CONSTRAINT ck_body_fat_tricep_gt_0 CHECK (tricep > 0),
	CONSTRAINT ck_body_fat_subscapular_gt_0 CHECK (subscapular > 0),
	CONSTRAINT ck_body_fat_suprailiac_gt_0 CHECK (suprailiac > 0),
	CONSTRAINT ck_body_fat_midaxillary_gt_0 CHECK (midaxillary > 0),
	CONSTRAINT fk_body_fat_user_id_user FOREIGN KEY(user_id) REFERENCES user (id) ON DELETE CASCADE
);
INSERT INTO "body_fat" VALUES(1,'2002-02-20',1,2,3,4,5,6,7);
INSERT INTO "body_fat" VALUES(1,'2002-02-21',NULL,NULL,10,11,NULL,13,NULL);
INSERT INTO "body_fat" VALUES(2,'2002-02-20',15,16,17,18,19,20,21);
INSERT INTO "body_fat" VALUES(2,'2002-02-22',22,23,24,25,26,27,28);
CREATE TABLE body_weight (
	user_id INTEGER NOT NULL,
	date DATE NOT NULL,
	weight FLOAT NOT NULL,
	CONSTRAINT pk_body_weight PRIMARY KEY (user_id, date),
	CONSTRAINT ck_body_weight_weight_type_real CHECK (typeof(weight) = 'real'),
	CONSTRAINT ck_body_weight_weight_gt_0 CHECK (weight > 0),
	CONSTRAINT fk_body_weight_user_id_user FOREIGN KEY(user_id) REFERENCES user (id) ON DELETE CASCADE
);
INSERT INTO "body_weight" VALUES(1,'2002-02-20',67.5);
INSERT INTO "body_weight" VALUES(1,'2002-02-21',67.7);
INSERT INTO "body_weight" VALUES(1,'2002-02-22',67.3);
INSERT INTO "body_weight" VALUES(2,'2002-02-20',100.0);
INSERT INTO "body_weight" VALUES(2,'2002-02-21',101.0);
INSERT INTO "body_weight" VALUES(2,'2002-02-22',102.0);
INSERT INTO "body_weight" VALUES(2,'2002-02-24',104.0);
INSERT INTO "body_weight" VALUES(2,'2002-02-25',105.0);
INSERT INTO "body_weight" VALUES(2,'2002-02-26',106.0);
INSERT INTO "body_weight" VALUES(2,'2002-02-28',108.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-01',109.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-02',110.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-03',111.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-05',113.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-06',114.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-07',115.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-08',116.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-10',118.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-11',119.0);
INSERT INTO "body_weight" VALUES(2,'2002-03-12',120.0);
CREATE TABLE exercise (
	id INTEGER NOT NULL,
	user_id INTEGER NOT NULL,
	name VARCHAR NOT NULL,
	CONSTRAINT pk_exercise PRIMARY KEY (id),
	CONSTRAINT uq_exercise_user_id UNIQUE (user_id, name),
	CONSTRAINT fk_exercise_user_id_user FOREIGN KEY(user_id) REFERENCES user (id) ON DELETE CASCADE
);
INSERT INTO "exercise" VALUES(1,1,'Exercise 1');
INSERT INTO "exercise" VALUES(2,2,'Exercise 2');
INSERT INTO "exercise" VALUES(3,1,'Exercise 2');
INSERT INTO "exercise" VALUES(4,2,'Exercise 3');
INSERT INTO "exercise" VALUES(5,1,'Unused Exercise');
CREATE TABLE period (
	user_id INTEGER NOT NULL,
	date DATE NOT NULL,
	intensity INTEGER NOT NULL,
	CONSTRAINT pk_period PRIMARY KEY (user_id, date),
	CONSTRAINT ck_period_intensity_type_integer CHECK (typeof(intensity) = 'integer'),
	CONSTRAINT ck_period_intensity_ge_1 CHECK (intensity >= 1),
	CONSTRAINT ck_period_intensity_le_4 CHECK (intensity <= 4),
	CONSTRAINT fk_period_user_id_user FOREIGN KEY(user_id) REFERENCES user (id) ON DELETE CASCADE
);
INSERT INTO "period" VALUES(1,'2002-02-20',2);
INSERT INTO "period" VALUES(1,'2002-02-21',4);
INSERT INTO "period" VALUES(1,'2002-02-22',1);
CREATE TABLE routine (
	id INTEGER NOT NULL,
	user_id INTEGER NOT NULL,
	name VARCHAR NOT NULL,
	notes VARCHAR,
	CONSTRAINT pk_routine PRIMARY KEY (id),
	CONSTRAINT uq_routine_user_id UNIQUE (user_id, name),
	CONSTRAINT fk_routine_user_id_user FOREIGN KEY(user_id) REFERENCES user (id) ON DELETE CASCADE
);
INSERT INTO "routine" VALUES(1,1,'R1','First Routine');
INSERT INTO "routine" VALUES(2,2,'R1',NULL);
INSERT INTO "routine" VALUES(3,1,'R2',NULL);
INSERT INTO "routine" VALUES(4,2,'Empty','TBD');
CREATE TABLE routine_activity (
	id INTEGER NOT NULL,
	exercise_id INTEGER,
	duration INTEGER NOT NULL,
	tempo INTEGER NOT NULL,
	automatic BOOLEAN NOT NULL,
	CONSTRAINT pk_routine_activity PRIMARY KEY (id),
	CONSTRAINT ck_routine_activity_duration_type_integer CHECK (typeof(duration) = 'integer'),
	CONSTRAINT ck_routine_activity_duration_ge_0 CHECK (duration >= 0),
	CONSTRAINT ck_routine_activity_tempo_type_integer CHECK (typeof(tempo) = 'integer'),
	CONSTRAINT ck_routine_activity_tempo_ge_0 CHECK (tempo >= 0),
	CONSTRAINT fk_routine_activity_id_routine_part FOREIGN KEY(id) REFERENCES routine_part (id),
	CONSTRAINT fk_routine_activity_exercise_id_exercise FOREIGN KEY(exercise_id) REFERENCES exercise (id) ON DELETE CASCADE
);
INSERT INTO "routine_activity" VALUES(5,1,0,3,0);
INSERT INTO "routine_activity" VALUES(6,NULL,60,0,0);
INSERT INTO "routine_activity" VALUES(8,3,0,0,0);
INSERT INTO "routine_activity" VALUES(9,NULL,30,0,0);
INSERT INTO "routine_activity" VALUES(10,3,20,0,1);
INSERT INTO "routine_activity" VALUES(11,NULL,10,0,1);
INSERT INTO "routine_activity" VALUES(12,3,20,0,1);
INSERT INTO "routine_activity" VALUES(13,NULL,10,0,1);
INSERT INTO "routine_activity" VALUES(14,NULL,30,0,0);
INSERT INTO "routine_activity" VALUES(15,1,0,0,0);
CREATE TABLE routine_part (
	id INTEGER NOT NULL,
	type VARCHAR NOT NULL,
	routine_section_id INTEGER,
	position INTEGER NOT NULL,
	CONSTRAINT pk_routine_part PRIMARY KEY (id),
	CONSTRAINT ck_routine_part_position_type_integer CHECK (typeof(position) = 'integer'),
	CONSTRAINT ck_routine_part_type_type_text CHECK (typeof(type) = 'text'),
	CONSTRAINT ck_routine_part_position_gt_0 CHECK (position > 0),
	CONSTRAINT fk_routine_part_routine_section_id_routine_section FOREIGN KEY(routine_section_id) REFERENCES routine_section (id) ON DELETE CASCADE
);
INSERT INTO "routine_part" VALUES(1,'routine_section',NULL,2);
INSERT INTO "routine_part" VALUES(2,'routine_section',NULL,1);
INSERT INTO "routine_part" VALUES(3,'routine_section',NULL,3);
INSERT INTO "routine_part" VALUES(4,'routine_section',NULL,1);
INSERT INTO "routine_part" VALUES(5,'routine_activity',1,1);
INSERT INTO "routine_part" VALUES(6,'routine_activity',1,2);
INSERT INTO "routine_part" VALUES(7,'routine_section',1,3);
INSERT INTO "routine_part" VALUES(8,'routine_activity',2,1);
INSERT INTO "routine_part" VALUES(9,'routine_activity',2,2);
INSERT INTO "routine_part" VALUES(10,'routine_activity',3,1);
INSERT INTO "routine_part" VALUES(11,'routine_activity',3,2);
INSERT INTO "routine_part" VALUES(12,'routine_activity',4,1);
INSERT INTO "routine_part" VALUES(13,'routine_activity',4,2);
INSERT INTO "routine_part" VALUES(14,'routine_activity',7,2);
INSERT INTO "routine_part" VALUES(15,'routine_activity',7,1);
CREATE TABLE routine_section (
	id INTEGER NOT NULL,
	routine_id INTEGER,
	rounds INTEGER NOT NULL,
	CONSTRAINT pk_routine_section PRIMARY KEY (id),
	CONSTRAINT ck_routine_section_rounds_type_integer CHECK (typeof(rounds) = 'integer'),
	CONSTRAINT ck_routine_section_rounds_gt_0 CHECK (rounds > 0),
	CONSTRAINT fk_routine_section_id_routine_part FOREIGN KEY(id) REFERENCES routine_part (id),
	CONSTRAINT fk_routine_section_routine_id_routine FOREIGN KEY(routine_id) REFERENCES routine (id) ON DELETE CASCADE
);
INSERT INTO "routine_section" VALUES(1,1,2);
INSERT INTO "routine_section" VALUES(2,1,1);
INSERT INTO "routine_section" VALUES(3,1,3);
INSERT INTO "routine_section" VALUES(4,3,5);
INSERT INTO "routine_section" VALUES(7,NULL,2);
CREATE TABLE user (
	id INTEGER NOT NULL,
	name VARCHAR NOT NULL,
	sex VARCHAR(6) NOT NULL,
	CONSTRAINT pk_user PRIMARY KEY (id),
	CONSTRAINT uq_user_name UNIQUE (name)
);
INSERT INTO "user" VALUES(1,'Alice','FEMALE');
INSERT INTO "user" VALUES(2,'Bob','MALE');
CREATE TABLE workout (
	id INTEGER NOT NULL,
	user_id INTEGER NOT NULL,
	routine_id INTEGER,
	date DATE NOT NULL,
	notes VARCHAR,
	CONSTRAINT pk_workout PRIMARY KEY (id),
	CONSTRAINT fk_workout_user_id_user FOREIGN KEY(user_id) REFERENCES user (id) ON DELETE CASCADE,
	CONSTRAINT fk_workout_routine_id_routine FOREIGN KEY(routine_id) REFERENCES routine (id) ON DELETE CASCADE
);
INSERT INTO "workout" VALUES(1,1,1,'2002-02-20','First Workout');
INSERT INTO "workout" VALUES(2,2,NULL,'2002-02-20',NULL);
INSERT INTO "workout" VALUES(3,1,NULL,'2002-02-22',NULL);
INSERT INTO "workout" VALUES(4,1,1,'2002-02-24',NULL);
CREATE TABLE workout_set (
	workout_id INTEGER NOT NULL,
	position INTEGER NOT NULL,
	exercise_id INTEGER NOT NULL,
	reps INTEGER,
	time INTEGER,
	weight FLOAT,
	rpe FLOAT,
	CONSTRAINT pk_workout_set PRIMARY KEY (workout_id, position),
	CONSTRAINT ck_workout_set_position_type_integer CHECK (typeof(position) = 'integer'),
	CONSTRAINT ck_workout_set_reps_type_integer_or_null CHECK (typeof(reps) = 'integer' or typeof(reps) = 'null'),
	CONSTRAINT ck_workout_set_time_type_integer_or_null CHECK (typeof(time) = 'integer' or typeof(time) = 'null'),
	CONSTRAINT ck_workout_set_weight_type_real_or_null CHECK (typeof(weight) = 'real' or typeof(weight) = 'null'),
	CONSTRAINT ck_workout_set_rpe_type_real_or_null CHECK (typeof(rpe) = 'real' or typeof(rpe) = 'null'),
	CONSTRAINT ck_workout_set_position_gt_0 CHECK (position > 0),
	CONSTRAINT ck_workout_set_reps_gt_0 CHECK (reps > 0),
	CONSTRAINT ck_workout_set_time_gt_0 CHECK (time > 0),
	CONSTRAINT ck_workout_set_weight_gt_0 CHECK (weight > 0),
	CONSTRAINT ck_workout_set_rpe_ge_0 CHECK (rpe >= 0),
	CONSTRAINT ck_workout_set_rpe_le_10 CHECK (rpe <= 10),
	CONSTRAINT fk_workout_set_workout_id_workout FOREIGN KEY(workout_id) REFERENCES workout (id) ON DELETE CASCADE,
	CONSTRAINT fk_workout_set_exercise_id_exercise FOREIGN KEY(exercise_id) REFERENCES exercise (id) ON DELETE CASCADE
);
INSERT INTO "workout_set" VALUES(1,2,1,9,4,NULL,8.5);
INSERT INTO "workout_set" VALUES(1,1,3,10,4,NULL,8.0);
INSERT INTO "workout_set" VALUES(1,3,1,NULL,60,NULL,9.0);
INSERT INTO "workout_set" VALUES(4,2,1,9,4,NULL,8.0);
INSERT INTO "workout_set" VALUES(4,1,3,11,4,NULL,8.5);
INSERT INTO "workout_set" VALUES(4,3,1,NULL,60,NULL,8.5);
INSERT INTO "workout_set" VALUES(3,1,3,9,NULL,NULL,NULL);
INSERT INTO "workout_set" VALUES(3,2,3,8,NULL,NULL,NULL);
INSERT INTO "workout_set" VALUES(3,3,3,7,NULL,NULL,NULL);
INSERT INTO "workout_set" VALUES(3,4,3,6,NULL,NULL,NULL);
INSERT INTO "workout_set" VALUES(3,5,3,5,NULL,NULL,NULL);
INSERT INTO "workout_set" VALUES(2,1,2,10,4,10.0,8.5);
INSERT INTO "workout_set" VALUES(2,4,4,7,NULL,7.5,NULL);
INSERT INTO "workout_set" VALUES(2,5,4,6,NULL,7.5,NULL);
INSERT INTO "workout_set" VALUES(2,6,4,5,NULL,7.5,NULL);
INSERT INTO "workout_set" VALUES(2,7,4,4,NULL,7.5,NULL);
INSERT INTO "workout_set" VALUES(2,2,2,9,4,8.0,9.0);
INSERT INTO "workout_set" VALUES(2,3,2,8,4,6.0,9.5);
COMMIT;
