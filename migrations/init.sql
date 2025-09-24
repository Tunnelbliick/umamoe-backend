--
-- PostgreSQL database dump
--

-- Dumped from database version 17.5
-- Dumped by pg_dump version 17.5

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: btree_gin; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS btree_gin WITH SCHEMA public;


--
-- Name: EXTENSION btree_gin; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION btree_gin IS 'support for indexing common datatypes in GIN';


--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


--
-- Name: increment_daily_inheritance_uploads(date); Type: FUNCTION; Schema: public; Owner: honsemoe
--

CREATE FUNCTION public.increment_daily_inheritance_uploads(target_date date) RETURNS integer
    LANGUAGE plpgsql
    AS $$
DECLARE
    current_count INTEGER;
BEGIN
    INSERT INTO daily_stats (date, total_visitors, unique_visitors, inheritance_uploads, support_card_uploads)
    VALUES (target_date, 0, 0, 1, 0)
    ON CONFLICT (date) 
    DO UPDATE SET 
        inheritance_uploads = daily_stats.inheritance_uploads + 1,
        updated_at = NOW()
    RETURNING inheritance_uploads INTO current_count;
    
    RETURN current_count;
END;
$$;


ALTER FUNCTION public.increment_daily_inheritance_uploads(target_date date) OWNER TO honsemoe;

--
-- Name: FUNCTION increment_daily_inheritance_uploads(target_date date); Type: COMMENT; Schema: public; Owner: honsemoe
--

COMMENT ON FUNCTION public.increment_daily_inheritance_uploads(target_date date) IS 'Atomically increments daily inheritance upload count';


--
-- Name: increment_daily_support_card_uploads(date); Type: FUNCTION; Schema: public; Owner: honsemoe
--

CREATE FUNCTION public.increment_daily_support_card_uploads(target_date date) RETURNS integer
    LANGUAGE plpgsql
    AS $$
DECLARE
    current_count INTEGER;
BEGIN
    INSERT INTO daily_stats (date, total_visitors, unique_visitors, inheritance_uploads, support_card_uploads)
    VALUES (target_date, 0, 0, 0, 1)
    ON CONFLICT (date) 
    DO UPDATE SET 
        support_card_uploads = daily_stats.support_card_uploads + 1,
        updated_at = NOW()
    RETURNING support_card_uploads INTO current_count;
    
    RETURN current_count;
END;
$$;


ALTER FUNCTION public.increment_daily_support_card_uploads(target_date date) OWNER TO honsemoe;

--
-- Name: FUNCTION increment_daily_support_card_uploads(target_date date); Type: COMMENT; Schema: public; Owner: honsemoe
--

COMMENT ON FUNCTION public.increment_daily_support_card_uploads(target_date date) IS 'Atomically increments daily support card upload count';


--
-- Name: increment_daily_visitor_count(date); Type: FUNCTION; Schema: public; Owner: honsemoe
--

CREATE FUNCTION public.increment_daily_visitor_count(target_date date) RETURNS integer
    LANGUAGE plpgsql
    AS $$
DECLARE
    current_count INTEGER;
BEGIN
    -- Use INSERT ... ON CONFLICT to handle upsert efficiently
    INSERT INTO daily_stats (date, total_visitors, unique_visitors, inheritance_uploads, support_card_uploads)
    VALUES (target_date, 1, 1, 0, 0)
    ON CONFLICT (date) 
    DO UPDATE SET 
        total_visitors = daily_stats.total_visitors + 1,
        unique_visitors = daily_stats.unique_visitors + 1,
        updated_at = NOW()
    RETURNING total_visitors INTO current_count;
    
    RETURN current_count;
END;
$$;


ALTER FUNCTION public.increment_daily_visitor_count(target_date date) OWNER TO honsemoe;

--
-- Name: FUNCTION increment_daily_visitor_count(target_date date); Type: COMMENT; Schema: public; Owner: honsemoe
--

COMMENT ON FUNCTION public.increment_daily_visitor_count(target_date date) IS 'Atomically increments daily visitor count, handles concurrent access safely';


--
-- Name: update_friendlist_reports_count(); Type: FUNCTION; Schema: public; Owner: honsemoe
--

CREATE FUNCTION public.update_friendlist_reports_count() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- This would update a count field if we had one
    -- For now, we'll just count reports in queries
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.update_friendlist_reports_count() OWNER TO honsemoe;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO honsemoe;

--
-- Name: accounts; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.accounts (
    account_id character varying(255) NOT NULL,
    viewer_id bigint NOT NULL,
    device_id character varying(255) NOT NULL,
    api_calls_per_minute integer DEFAULT 10,
    is_active boolean DEFAULT true,
    success_rate numeric(5,2) DEFAULT 100.0,
    last_used timestamp without time zone,
    api_calls_today integer DEFAULT 0,
    steam_session bytea,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.accounts OWNER TO honsemoe;

--
-- Name: blue_factors; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.blue_factors (
    record_id uuid NOT NULL,
    factor_type character varying(20) NOT NULL,
    level integer NOT NULL,
    CONSTRAINT blue_factors_factor_type_check CHECK (((factor_type)::text = ANY (ARRAY[('Speed'::character varying)::text, ('Stamina'::character varying)::text, ('Power'::character varying)::text, ('Guts'::character varying)::text, ('Wit'::character varying)::text]))),
    CONSTRAINT blue_factors_level_check CHECK (((level >= 1) AND (level <= 9)))
);


ALTER TABLE public.blue_factors OWNER TO honsemoe;

--
-- Name: circles; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.circles (
    circle_id bigint NOT NULL,
    name character varying(255) NOT NULL,
    comment text,
    leader_viewer_id bigint,
    member_count integer,
    join_style integer,
    policy integer,
    created_at timestamp without time zone,
    last_updated timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    monthly_rank integer,
    monthly_point bigint,
    last_month_rank integer,
    last_month_point bigint
);


ALTER TABLE public.circles OWNER TO honsemoe;

--
-- Name: daily_stats; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.daily_stats (
    id integer NOT NULL,
    date date NOT NULL,
    total_visitors integer DEFAULT 0,
    unique_visitors integer DEFAULT 0,
    inheritance_uploads integer DEFAULT 0,
    support_card_uploads integer DEFAULT 0,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now(),
    visitor_count integer DEFAULT 0
);


ALTER TABLE public.daily_stats OWNER TO honsemoe;

--
-- Name: TABLE daily_stats; Type: COMMENT; Schema: public; Owner: honsemoe
--

COMMENT ON TABLE public.daily_stats IS 'Efficient daily aggregated stats - one row per day instead of individual visit tracking';


--
-- Name: daily_stats_id_seq; Type: SEQUENCE; Schema: public; Owner: honsemoe
--

CREATE SEQUENCE public.daily_stats_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.daily_stats_id_seq OWNER TO honsemoe;

--
-- Name: daily_stats_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: honsemoe
--

ALTER SEQUENCE public.daily_stats_id_seq OWNED BY public.daily_stats.id;


--
-- Name: daily_visitor_counters; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.daily_visitor_counters (
    date date NOT NULL,
    visitor_count integer DEFAULT 0,
    created_at timestamp with time zone DEFAULT now(),
    updated_at timestamp with time zone DEFAULT now()
);


ALTER TABLE public.daily_visitor_counters OWNER TO honsemoe;

--
-- Name: friendlist_reports; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.friendlist_reports (
    trainer_id character varying(15) NOT NULL,
    reported_at timestamp with time zone DEFAULT now(),
    report_count integer DEFAULT 1,
    CONSTRAINT trainer_id_format_fl CHECK (((trainer_id)::text ~ '^[0-9 ]+$'::text))
);


ALTER TABLE public.friendlist_reports OWNER TO honsemoe;

--
-- Name: inheritance; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.inheritance (
    inheritance_id integer NOT NULL,
    account_id character varying(255) NOT NULL,
    main_parent_id integer DEFAULT 0 NOT NULL,
    parent_left_id integer DEFAULT 0 NOT NULL,
    parent_right_id integer DEFAULT 0 NOT NULL,
    blue_sparks integer[] DEFAULT '{}'::integer[],
    pink_sparks integer[] DEFAULT '{}'::integer[],
    green_sparks integer[] DEFAULT '{}'::integer[],
    white_sparks integer[] DEFAULT '{}'::integer[],
    win_count integer DEFAULT 0,
    white_count integer DEFAULT 0,
    parent_rank integer DEFAULT 0 NOT NULL,
    parent_rarity integer DEFAULT 0 NOT NULL,
    main_blue_factors integer DEFAULT 0,
    main_pink_factors integer DEFAULT 0,
    main_green_factors integer DEFAULT 0,
    main_white_factors integer[] DEFAULT '{}'::integer[],
    main_white_count integer DEFAULT 0
);


ALTER TABLE public.inheritance OWNER TO honsemoe;

--
-- Name: inheritance_id_seq; Type: SEQUENCE; Schema: public; Owner: honsemoe
--

CREATE SEQUENCE public.inheritance_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.inheritance_id_seq OWNER TO honsemoe;

--
-- Name: inheritance_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: honsemoe
--

ALTER SEQUENCE public.inheritance_id_seq OWNED BY public.inheritance.inheritance_id;


--
-- Name: inheritance_records; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.inheritance_records (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    trainer_id character varying(15) NOT NULL,
    main_character_id integer NOT NULL,
    parent1_id integer NOT NULL,
    parent2_id integer NOT NULL,
    submitted_at timestamp with time zone DEFAULT now(),
    verified boolean DEFAULT false,
    upvotes integer DEFAULT 0,
    downvotes integer DEFAULT 0,
    notes text,
    CONSTRAINT trainer_id_format CHECK (((trainer_id)::text ~ '^[0-9 ]+$'::text))
);


ALTER TABLE public.inheritance_records OWNER TO honsemoe;

--
-- Name: pink_factors; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.pink_factors (
    record_id uuid NOT NULL,
    factor_type character varying(20) NOT NULL,
    level integer NOT NULL,
    CONSTRAINT pink_factors_factor_type_check CHECK (((factor_type)::text = ANY (ARRAY[('Turf'::character varying)::text, ('Dirt'::character varying)::text, ('Sprint'::character varying)::text, ('Mile'::character varying)::text, ('Middle'::character varying)::text, ('Long'::character varying)::text, ('Front Runner'::character varying)::text, ('Pace Chaser'::character varying)::text, ('Late Surger'::character varying)::text, ('End'::character varying)::text]))),
    CONSTRAINT pink_factors_level_check CHECK (((level >= 1) AND (level <= 9)))
);


ALTER TABLE public.pink_factors OWNER TO honsemoe;

--
-- Name: support_card; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.support_card (
    account_id character varying(255) NOT NULL,
    support_card_id integer NOT NULL,
    limit_break_count integer DEFAULT 0,
    experience integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.support_card OWNER TO honsemoe;

--
-- Name: support_card_records; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.support_card_records (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    trainer_id character varying(15) NOT NULL,
    card_id character varying(50) NOT NULL,
    limit_break integer NOT NULL,
    rarity integer NOT NULL,
    card_type integer NOT NULL,
    submitted_at timestamp with time zone DEFAULT now(),
    upvotes integer DEFAULT 0,
    downvotes integer DEFAULT 0,
    CONSTRAINT support_card_records_card_type_check CHECK (((card_type >= 0) AND (card_type <= 6))),
    CONSTRAINT support_card_records_limit_break_check CHECK (((limit_break >= 0) AND (limit_break <= 4))),
    CONSTRAINT support_card_records_rarity_check CHECK (((rarity >= 1) AND (rarity <= 3))),
    CONSTRAINT trainer_id_format CHECK (((trainer_id)::text ~ '^[0-9]+$'::text))
);


ALTER TABLE public.support_card_records OWNER TO honsemoe;

--
-- Name: tasks; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.tasks (
    id integer NOT NULL,
    task_type character varying(255) NOT NULL,
    task_data jsonb NOT NULL,
    priority integer DEFAULT 0,
    status character varying(50) DEFAULT 'pending'::character varying,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    worker_id character varying(255),
    error_message text,
    account_id character varying(255),
    retry_count integer DEFAULT 0,
    max_retries integer DEFAULT 3
);


ALTER TABLE public.tasks OWNER TO honsemoe;

--
-- Name: tasks_id_seq; Type: SEQUENCE; Schema: public; Owner: honsemoe
--

CREATE SEQUENCE public.tasks_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.tasks_id_seq OWNER TO honsemoe;

--
-- Name: tasks_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: honsemoe
--

ALTER SEQUENCE public.tasks_id_seq OWNED BY public.tasks.id;


--
-- Name: team_stadium; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.team_stadium (
    id integer NOT NULL,
    trainer_id character varying(255) NOT NULL,
    distance_type integer NOT NULL,
    member_id integer NOT NULL,
    trained_chara_id integer NOT NULL,
    running_style integer NOT NULL,
    card_id bigint NOT NULL,
    speed integer DEFAULT 0 NOT NULL,
    power integer DEFAULT 0 NOT NULL,
    stamina integer DEFAULT 0 NOT NULL,
    wiz integer DEFAULT 0 NOT NULL,
    guts integer DEFAULT 0 NOT NULL,
    fans integer DEFAULT 0 NOT NULL,
    rank_score integer DEFAULT 0 NOT NULL,
    skills integer[] DEFAULT '{}'::integer[],
    creation_time timestamp without time zone NOT NULL,
    scenario_id integer DEFAULT 0 NOT NULL,
    factors integer[] DEFAULT '{}'::integer[],
    support_cards integer[] DEFAULT '{}'::integer[],
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    proper_ground_turf integer DEFAULT 0,
    proper_ground_dirt integer DEFAULT 0,
    proper_running_style_front integer DEFAULT 0,
    proper_running_style_pace integer DEFAULT 0,
    proper_running_style_late integer DEFAULT 0,
    proper_running_style_end integer DEFAULT 0,
    proper_distance_short integer DEFAULT 0,
    proper_distance_mile integer DEFAULT 0,
    proper_distance_middle integer DEFAULT 0,
    proper_distance_long integer DEFAULT 0,
    rarity integer DEFAULT 0,
    talent_level integer DEFAULT 0,
    proper_running_style_nige integer DEFAULT 0,
    proper_running_style_senko integer DEFAULT 0,
    proper_running_style_sashi integer DEFAULT 0,
    proper_running_style_oikomi integer DEFAULT 0
);


ALTER TABLE public.team_stadium OWNER TO honsemoe;

--
-- Name: team_stadium_characters; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.team_stadium_characters (
    id integer NOT NULL,
    trainer_id bigint NOT NULL,
    distance_type integer NOT NULL,
    member_id integer NOT NULL,
    trained_chara_id integer NOT NULL,
    running_style integer NOT NULL,
    card_id bigint NOT NULL,
    speed integer DEFAULT 0 NOT NULL,
    power integer DEFAULT 0 NOT NULL,
    stamina integer DEFAULT 0 NOT NULL,
    wiz integer DEFAULT 0 NOT NULL,
    guts integer DEFAULT 0 NOT NULL,
    fans integer DEFAULT 0 NOT NULL,
    rank_score integer DEFAULT 0 NOT NULL,
    skills text,
    creation_time timestamp without time zone NOT NULL,
    scenario_id integer DEFAULT 0 NOT NULL,
    factors text,
    support_cards text,
    created_at timestamp without time zone DEFAULT CURRENT_TIMESTAMP
);


ALTER TABLE public.team_stadium_characters OWNER TO honsemoe;

--
-- Name: team_stadium_characters_id_seq; Type: SEQUENCE; Schema: public; Owner: honsemoe
--

CREATE SEQUENCE public.team_stadium_characters_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.team_stadium_characters_id_seq OWNER TO honsemoe;

--
-- Name: team_stadium_characters_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: honsemoe
--

ALTER SEQUENCE public.team_stadium_characters_id_seq OWNED BY public.team_stadium_characters.id;


--
-- Name: team_stadium_id_seq; Type: SEQUENCE; Schema: public; Owner: honsemoe
--

CREATE SEQUENCE public.team_stadium_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.team_stadium_id_seq OWNER TO honsemoe;

--
-- Name: team_stadium_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: honsemoe
--

ALTER SEQUENCE public.team_stadium_id_seq OWNED BY public.team_stadium.id;


--
-- Name: trainer; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.trainer (
    account_id character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    follower_num integer DEFAULT 0,
    last_updated timestamp without time zone DEFAULT CURRENT_TIMESTAMP,
    circle_id bigint,
    circle_name character varying(255),
    circle_membership integer,
    fans bigint,
    best_team_class integer,
    team_class integer
);


ALTER TABLE public.trainer OWNER TO honsemoe;

--
-- Name: unique_skills; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.unique_skills (
    record_id uuid NOT NULL,
    skill_id integer NOT NULL,
    level integer NOT NULL,
    CONSTRAINT unique_skills_level_check CHECK (((level >= 1) AND (level <= 9)))
);


ALTER TABLE public.unique_skills OWNER TO honsemoe;

--
-- Name: votes; Type: TABLE; Schema: public; Owner: honsemoe
--

CREATE TABLE public.votes (
    id integer NOT NULL,
    record_id uuid NOT NULL,
    user_ip inet NOT NULL,
    vote_type character varying(10) NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    CONSTRAINT votes_vote_type_check CHECK (((vote_type)::text = ANY (ARRAY[('up'::character varying)::text, ('down'::character varying)::text])))
);


ALTER TABLE public.votes OWNER TO honsemoe;

--
-- Name: votes_id_seq; Type: SEQUENCE; Schema: public; Owner: honsemoe
--

CREATE SEQUENCE public.votes_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER SEQUENCE public.votes_id_seq OWNER TO honsemoe;

--
-- Name: votes_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: honsemoe
--

ALTER SEQUENCE public.votes_id_seq OWNED BY public.votes.id;


--
-- Name: daily_stats id; Type: DEFAULT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.daily_stats ALTER COLUMN id SET DEFAULT nextval('public.daily_stats_id_seq'::regclass);


--
-- Name: inheritance inheritance_id; Type: DEFAULT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.inheritance ALTER COLUMN inheritance_id SET DEFAULT nextval('public.inheritance_id_seq'::regclass);


--
-- Name: tasks id; Type: DEFAULT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.tasks ALTER COLUMN id SET DEFAULT nextval('public.tasks_id_seq'::regclass);


--
-- Name: team_stadium id; Type: DEFAULT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.team_stadium ALTER COLUMN id SET DEFAULT nextval('public.team_stadium_id_seq'::regclass);


--
-- Name: team_stadium_characters id; Type: DEFAULT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.team_stadium_characters ALTER COLUMN id SET DEFAULT nextval('public.team_stadium_characters_id_seq'::regclass);


--
-- Name: votes id; Type: DEFAULT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.votes ALTER COLUMN id SET DEFAULT nextval('public.votes_id_seq'::regclass);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: accounts accounts_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.accounts
    ADD CONSTRAINT accounts_pkey PRIMARY KEY (account_id);


--
-- Name: accounts accounts_viewer_id_key; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.accounts
    ADD CONSTRAINT accounts_viewer_id_key UNIQUE (viewer_id);


--
-- Name: blue_factors blue_factors_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.blue_factors
    ADD CONSTRAINT blue_factors_pkey PRIMARY KEY (record_id, factor_type);


--
-- Name: circles circles_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.circles
    ADD CONSTRAINT circles_pkey PRIMARY KEY (circle_id);


--
-- Name: daily_stats daily_stats_date_key; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.daily_stats
    ADD CONSTRAINT daily_stats_date_key UNIQUE (date);


--
-- Name: daily_stats daily_stats_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.daily_stats
    ADD CONSTRAINT daily_stats_pkey PRIMARY KEY (id);


--
-- Name: daily_visitor_counters daily_visitor_counters_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.daily_visitor_counters
    ADD CONSTRAINT daily_visitor_counters_pkey PRIMARY KEY (date);


--
-- Name: friendlist_reports friendlist_reports_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.friendlist_reports
    ADD CONSTRAINT friendlist_reports_pkey PRIMARY KEY (trainer_id);


--
-- Name: inheritance inheritance_account_id_key; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.inheritance
    ADD CONSTRAINT inheritance_account_id_key UNIQUE (account_id);


--
-- Name: inheritance inheritance_account_id_unique; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.inheritance
    ADD CONSTRAINT inheritance_account_id_unique UNIQUE (account_id);


--
-- Name: inheritance inheritance_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.inheritance
    ADD CONSTRAINT inheritance_pkey PRIMARY KEY (inheritance_id);


--
-- Name: inheritance_records inheritance_records_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.inheritance_records
    ADD CONSTRAINT inheritance_records_pkey PRIMARY KEY (id);


--
-- Name: pink_factors pink_factors_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.pink_factors
    ADD CONSTRAINT pink_factors_pkey PRIMARY KEY (record_id, factor_type);


--
-- Name: support_card support_card_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.support_card
    ADD CONSTRAINT support_card_pkey PRIMARY KEY (account_id, support_card_id);


--
-- Name: support_card_records support_card_records_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.support_card_records
    ADD CONSTRAINT support_card_records_pkey PRIMARY KEY (id);


--
-- Name: tasks tasks_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);


--
-- Name: team_stadium_characters team_stadium_characters_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.team_stadium_characters
    ADD CONSTRAINT team_stadium_characters_pkey PRIMARY KEY (id);


--
-- Name: team_stadium_characters team_stadium_characters_trainer_id_distance_type_member_id_key; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.team_stadium_characters
    ADD CONSTRAINT team_stadium_characters_trainer_id_distance_type_member_id_key UNIQUE (trainer_id, distance_type, member_id);


--
-- Name: team_stadium team_stadium_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.team_stadium
    ADD CONSTRAINT team_stadium_pkey PRIMARY KEY (id);


--
-- Name: team_stadium team_stadium_trainer_id_distance_type_member_id_key; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.team_stadium
    ADD CONSTRAINT team_stadium_trainer_id_distance_type_member_id_key UNIQUE (trainer_id, distance_type, member_id);


--
-- Name: trainer trainer_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.trainer
    ADD CONSTRAINT trainer_pkey PRIMARY KEY (account_id);


--
-- Name: unique_skills unique_skills_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.unique_skills
    ADD CONSTRAINT unique_skills_pkey PRIMARY KEY (record_id, skill_id);


--
-- Name: support_card_records unique_submission; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.support_card_records
    ADD CONSTRAINT unique_submission UNIQUE (trainer_id, card_id, limit_break);


--
-- Name: votes votes_pkey; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.votes
    ADD CONSTRAINT votes_pkey PRIMARY KEY (id);


--
-- Name: votes votes_record_id_user_ip_key; Type: CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.votes
    ADD CONSTRAINT votes_record_id_user_ip_key UNIQUE (record_id, user_ip);


--
-- Name: idx_accounts_active; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_accounts_active ON public.accounts USING btree (is_active);


--
-- Name: idx_blue_factors_composite; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_blue_factors_composite ON public.blue_factors USING btree (factor_type, level, record_id);


--
-- Name: idx_blue_factors_record; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_blue_factors_record ON public.blue_factors USING btree (record_id);


--
-- Name: idx_blue_factors_type_level; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_blue_factors_type_level ON public.blue_factors USING btree (factor_type, level);


--
-- Name: idx_circle_leader; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_circle_leader ON public.circles USING btree (leader_viewer_id);


--
-- Name: idx_circle_updated; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_circle_updated ON public.circles USING btree (last_updated);


--
-- Name: idx_daily_stats_date; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_daily_stats_date ON public.daily_stats USING btree (date);


--
-- Name: idx_daily_stats_date_performance; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_daily_stats_date_performance ON public.daily_stats USING btree (date DESC);


--
-- Name: idx_daily_visitor_counters_date; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_daily_visitor_counters_date ON public.daily_visitor_counters USING btree (date);


--
-- Name: idx_friendlist_reports_reported_at; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_friendlist_reports_reported_at ON public.friendlist_reports USING btree (reported_at);


--
-- Name: idx_friendlist_reports_trainer; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_friendlist_reports_trainer ON public.friendlist_reports USING btree (trainer_id);


--
-- Name: idx_inheritance_blue_sparks; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_blue_sparks ON public.inheritance USING gin (blue_sparks);


--
-- Name: idx_inheritance_green_sparks; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_green_sparks ON public.inheritance USING gin (green_sparks);


--
-- Name: idx_inheritance_main_character; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_main_character ON public.inheritance_records USING btree (main_character_id);


--
-- Name: idx_inheritance_parents; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_parents ON public.inheritance_records USING btree (parent1_id, parent2_id);


--
-- Name: idx_inheritance_pink_sparks; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_pink_sparks ON public.inheritance USING gin (pink_sparks);


--
-- Name: idx_inheritance_submitted; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_submitted ON public.inheritance_records USING btree (submitted_at DESC);


--
-- Name: idx_inheritance_trainer; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_trainer ON public.inheritance_records USING btree (trainer_id);


--
-- Name: idx_inheritance_trainer_id; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_trainer_id ON public.inheritance USING btree (account_id);


--
-- Name: idx_inheritance_verified; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_verified ON public.inheritance_records USING btree (verified);


--
-- Name: idx_inheritance_white_count; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_white_count ON public.inheritance USING btree (white_count DESC);


--
-- Name: idx_inheritance_white_sparks; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_white_sparks ON public.inheritance USING gin (white_sparks);


--
-- Name: idx_inheritance_win_count; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_inheritance_win_count ON public.inheritance USING btree (win_count DESC);


--
-- Name: idx_pink_factors_composite; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_pink_factors_composite ON public.pink_factors USING btree (factor_type, level, record_id);


--
-- Name: idx_pink_factors_record; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_pink_factors_record ON public.pink_factors USING btree (record_id);


--
-- Name: idx_pink_factors_type_level; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_pink_factors_type_level ON public.pink_factors USING btree (factor_type, level);


--
-- Name: idx_support_card_records_card_id; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_card_id ON public.support_card_records USING btree (card_id);


--
-- Name: idx_support_card_records_limit_break; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_limit_break ON public.support_card_records USING btree (limit_break);


--
-- Name: idx_support_card_records_submitted_at; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_submitted_at ON public.support_card_records USING btree (submitted_at);


--
-- Name: idx_support_card_records_trainer_id; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_trainer_id ON public.support_card_records USING btree (trainer_id);


--
-- Name: idx_support_card_records_type_rarity; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_type_rarity ON public.support_card_records USING btree (card_type, rarity);


--
-- Name: idx_support_card_records_type_rarity_lb; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_type_rarity_lb ON public.support_card_records USING btree (card_type, rarity, limit_break);


--
-- Name: idx_support_card_records_upvotes; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_support_card_records_upvotes ON public.support_card_records USING btree (upvotes DESC);


--
-- Name: idx_tasks_priority; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_tasks_priority ON public.tasks USING btree (priority DESC);


--
-- Name: idx_tasks_status; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_tasks_status ON public.tasks USING btree (status);


--
-- Name: idx_team_stadium_chara; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_chara ON public.team_stadium_characters USING btree (trained_chara_id);


--
-- Name: idx_team_stadium_creation; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_creation ON public.team_stadium_characters USING btree (creation_time);


--
-- Name: idx_team_stadium_distance; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_distance ON public.team_stadium_characters USING btree (distance_type);


--
-- Name: idx_team_stadium_factors_gin; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_factors_gin ON public.team_stadium_characters USING gin (((factors)::jsonb));


--
-- Name: idx_team_stadium_rank; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_rank ON public.team_stadium_characters USING btree (rank_score);


--
-- Name: idx_team_stadium_scenario; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_scenario ON public.team_stadium_characters USING btree (scenario_id);


--
-- Name: idx_team_stadium_skills_gin; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_skills_gin ON public.team_stadium_characters USING gin (((skills)::jsonb));


--
-- Name: idx_team_stadium_stats; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_stats ON public.team_stadium_characters USING btree (speed, power, stamina, wiz, guts);


--
-- Name: idx_team_stadium_style; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_style ON public.team_stadium_characters USING btree (running_style);


--
-- Name: idx_team_stadium_support_gin; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_support_gin ON public.team_stadium_characters USING gin (((support_cards)::jsonb));


--
-- Name: idx_team_stadium_trainer; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_team_stadium_trainer ON public.team_stadium_characters USING btree (trainer_id);


--
-- Name: idx_trainer_circle; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_trainer_circle ON public.trainer USING btree (circle_id);


--
-- Name: idx_trainer_updated; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_trainer_updated ON public.trainer USING btree (last_updated);


--
-- Name: idx_trainers_follower_num; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_trainers_follower_num ON public.trainer USING btree (follower_num DESC);


--
-- Name: idx_unique_skills_composite; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_unique_skills_composite ON public.unique_skills USING btree (skill_id, level, record_id);


--
-- Name: idx_unique_skills_record; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_unique_skills_record ON public.unique_skills USING btree (record_id);


--
-- Name: idx_unique_skills_skill_level; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_unique_skills_skill_level ON public.unique_skills USING btree (skill_id, level);


--
-- Name: idx_votes_record; Type: INDEX; Schema: public; Owner: honsemoe
--

CREATE INDEX idx_votes_record ON public.votes USING btree (record_id);


--
-- Name: friendlist_reports friendlist_reports_trigger; Type: TRIGGER; Schema: public; Owner: honsemoe
--

CREATE TRIGGER friendlist_reports_trigger AFTER INSERT ON public.friendlist_reports FOR EACH ROW EXECUTE FUNCTION public.update_friendlist_reports_count();


--
-- Name: blue_factors blue_factors_record_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.blue_factors
    ADD CONSTRAINT blue_factors_record_id_fkey FOREIGN KEY (record_id) REFERENCES public.inheritance_records(id) ON DELETE CASCADE;


--
-- Name: inheritance inheritance_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.inheritance
    ADD CONSTRAINT inheritance_account_id_fkey FOREIGN KEY (account_id) REFERENCES public.trainer(account_id) ON DELETE CASCADE;


--
-- Name: pink_factors pink_factors_record_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.pink_factors
    ADD CONSTRAINT pink_factors_record_id_fkey FOREIGN KEY (record_id) REFERENCES public.inheritance_records(id) ON DELETE CASCADE;


--
-- Name: support_card support_card_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.support_card
    ADD CONSTRAINT support_card_account_id_fkey FOREIGN KEY (account_id) REFERENCES public.trainer(account_id);


--
-- Name: tasks tasks_account_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.tasks
    ADD CONSTRAINT tasks_account_id_fkey FOREIGN KEY (account_id) REFERENCES public.accounts(account_id);


--
-- Name: unique_skills unique_skills_record_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.unique_skills
    ADD CONSTRAINT unique_skills_record_id_fkey FOREIGN KEY (record_id) REFERENCES public.inheritance_records(id) ON DELETE CASCADE;


--
-- Name: votes votes_record_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: honsemoe
--

ALTER TABLE ONLY public.votes
    ADD CONSTRAINT votes_record_id_fkey FOREIGN KEY (record_id) REFERENCES public.inheritance_records(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

