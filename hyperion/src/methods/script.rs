//! Definition of the Script method

use std::time::Instant;

use std::fs;
use std::path::Path;

use rlua::{Function, Lua};

use serde_yaml::Value;
use std::collections::BTreeMap as Map;

use super::{LedInstance, Method};

use crate::filters::ColorFilter;
use crate::runtime::IdleTracker;

/// Dummy LED device which outputs updates to the standard output
pub struct Script {
    /// Lua engine instance
    lua: Lua,
}

/// Scripting engine error
#[derive(Debug, Fail)]
pub enum ScriptError {
    /// Failed to load the script
    #[fail(display = "loading the script failed: {}", 0)]
    LoadError(failure::Error),
    /// An error ocurred inside the scripting engine
    #[fail(display = "lua error: {}", 0)]
    LuaError(rlua::Error),
}

impl From<rlua::Error> for ScriptError {
    fn from(lua_error: rlua::Error) -> Self {
        ScriptError::LuaError(lua_error)
    }
}

macro_rules! register_lua_log {
    ($lua_ctx:expr, $path:expr, $log:tt, $name:expr) => {{
        let cloned_log_path = $path.clone().into_owned();
        let log_function = $lua_ctx.create_function(move |_, message: String| {
            $log!("{}: {}", cloned_log_path, message);
            Ok(())
        })?;

        $lua_ctx.globals().set($name, log_function)?;
    }};
}

impl Script {
    /// Convert a serde_yaml::Value into a lua value
    ///
    /// # Parameters
    ///
    /// * `lua_ctx`: context to create Lua values in
    /// * `value`: value to convert
    fn lua_value<'lua>(
        lua_ctx: rlua::Context<'lua>,
        value: &Value,
    ) -> rlua::Result<rlua::Value<'lua>> {
        match value {
            Value::Null => Ok(rlua::Value::Nil),
            Value::Bool(bool_value) => Ok(rlua::Value::Boolean(*bool_value)),
            Value::Number(number_value) => {
                if number_value.is_i64() {
                    Ok(rlua::Value::Integer(number_value.as_i64().unwrap()))
                } else {
                    Ok(rlua::Value::Number(number_value.as_f64().unwrap()))
                }
            }
            Value::String(string_value) => {
                Ok(rlua::Value::String(lua_ctx.create_string(&string_value)?))
            }
            Value::Sequence(array_value) => {
                let table = lua_ctx.create_table()?;

                for (i, item) in array_value.iter().enumerate() {
                    table.set(i + 1, Self::lua_value(lua_ctx, item)?)?;
                }

                Ok(rlua::Value::Table(table))
            }
            Value::Mapping(object_value) => {
                let table = lua_ctx.create_table()?;

                for (k, item) in object_value.iter() {
                    // Ignore non-string keys
                    if let Some(key) = k.as_str() {
                        table.set(key, Self::lua_value(lua_ctx, item)?)?;
                    }
                }

                Ok(rlua::Value::Table(table))
            }
        }
    }

    /// Create a new script method
    ///
    /// # Parameters
    ///
    /// * `path`: path to the script to load
    /// * `params`: parameters to pass to the script context
    pub fn new<P: AsRef<Path>>(
        path: &P,
        params: Map<String, Value>,
    ) -> std::result::Result<Self, ScriptError> {
        let lua = Lua::new();
        let path = path.as_ref().to_path_buf();

        match lua.context(move |lua_ctx| -> std::result::Result<(), failure::Error> {
            // Create params table
            let params_table = lua_ctx.create_table()?;
            for (key, value) in params.iter() {
                params_table.set(key.to_string(), Self::lua_value(lua_ctx, value)?)?;
            }

            // Add host information
            let hyperion_table = lua_ctx.create_table()?;
            hyperion_table.set(
                "version",
                format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            )?;

            // Register table in the params table
            params_table.set("host", hyperion_table)?;

            let globals = lua_ctx.globals();

            // Register table
            globals.set("hyperion_params", params_table)?;

            // Create log functions
            let path_name = path.as_path().to_string_lossy();
            register_lua_log!(lua_ctx, path_name, debug, "pdebug");
            register_lua_log!(lua_ctx, path_name, error, "perror");
            register_lua_log!(lua_ctx, path_name, info, "pinfo");
            register_lua_log!(lua_ctx, path_name, trace, "ptrace");
            register_lua_log!(lua_ctx, path_name, warn, "pwarn");

            // Load script
            lua_ctx.load(&fs::read_to_string(path)?).exec()?;

            Ok(())
        }) {
            Ok(_) => Ok(Self { lua }),
            Err(error) => Err(ScriptError::LoadError(error)),
        }
    }
}

impl Method for Script {
    fn write(
        &self,
        time: Instant,
        filter: &ColorFilter,
        leds: &mut [LedInstance],
        idle_tracker: &mut IdleTracker,
    ) {
        self.lua
            .context(|lua_ctx| -> std::result::Result<(), ScriptError> {
                let globals = lua_ctx.globals();

                let write_function: Function = globals.get("write")?;

                let led_table = lua_ctx.create_table()?;
                for (i, led) in leds.iter_mut().enumerate() {
                    let color_data = lua_ctx.create_table()?;

                    let current_color = led.next_value(time, &filter, idle_tracker);
                    let (r, g, b) = current_color.into_components();
                    color_data.set("r", r)?;
                    color_data.set("g", g)?;
                    color_data.set("b", b)?;

                    led_table.set(i + 1, color_data)?;
                }

                write_function.call::<_, ()>(led_table)?;

                Ok(())
            })
            .expect("failed to write LED data");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Instant;

    use crate::config::{Filter, IdleSettings, Led};
    use crate::filters::ColorFilter;
    use crate::runtime::IdleTracker;

    #[test]
    fn script_method() {
        let method: Box<dyn Method> =
            Box::new(Script::new(&"../scripts/methods/stdout.lua", Map::new()).unwrap());

        let time = Instant::now();
        let filter = ColorFilter::from(Filter::default());
        let mut leds = vec![LedInstance::new(Led::default(), 1)];
        let mut idle_tracker = IdleTracker::from(IdleSettings::default());

        method.write(time, &filter, &mut leds[..], &mut idle_tracker);
    }
}
