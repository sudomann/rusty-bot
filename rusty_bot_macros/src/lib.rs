

#[derive(Default)]
struct ValidationConfig {
    require_pug_channel: bool,
    require_admin_privilege: bool,
    require_mod_privilege: bool,
    
    // custom_logic: Option<fn(&Context, &Message) -> anyhow::Result<()>>,
}