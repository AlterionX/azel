// @generated automatically by Diesel CLI.

diesel::table! {
    event_participation_count_changes (id) {
        id -> Int8,
        created -> Timestamptz,
        updater -> Numeric,
        target -> Numeric,
        event_participation -> Numeric,
        #[max_length = 10000]
        user_note -> Nullable<Varchar>,
    }
}

diesel::table! {
    event_participation_counts (id) {
        id -> Numeric,
        created -> Timestamptz,
        updated -> Timestamptz,
        event_participation -> Numeric,
    }
}

diesel::table! {
    industry_profit_count_changes (id) {
        id -> Int8,
        created -> Timestamptz,
        updater -> Numeric,
        target -> Numeric,
        alpha_united_earth_credits -> Numeric,
    }
}

diesel::table! {
    industry_profit_counts (id) {
        id -> Numeric,
        created -> Timestamptz,
        updated -> Timestamptz,
        alpha_united_earth_credits -> Numeric,
    }
}

diesel::table! {
    legion_kill_count_changes (id) {
        id -> Int8,
        created -> Timestamptz,
        updater -> Numeric,
        target -> Numeric,
        kills -> Numeric,
    }
}

diesel::table! {
    legion_kill_counts (id) {
        id -> Numeric,
        created -> Timestamptz,
        updated -> Timestamptz,
        kills -> Numeric,
    }
}

diesel::table! {
    naval_victory_count_changes (id) {
        id -> Int8,
        created -> Timestamptz,
        updater -> Numeric,
        target -> Numeric,
        victory_fourths -> Numeric,
    }
}

diesel::table! {
    naval_victory_counts (id) {
        id -> Numeric,
        created -> Timestamptz,
        updated -> Timestamptz,
        victory_fourths -> Numeric,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    event_participation_count_changes,
    event_participation_counts,
    industry_profit_count_changes,
    industry_profit_counts,
    legion_kill_count_changes,
    legion_kill_counts,
    naval_victory_count_changes,
    naval_victory_counts,
);
