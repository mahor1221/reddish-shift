/*  location-geoclue2.c -- GeoClue2 location provider
    This file is part of <https://github.com/mahor1221/reddish-shift>.
    Copyright (C) 2024 Mahor Foruzesh <mahor1221@gmail.com>
    Ported from Redshift <https://github.com/jonls/redshift>.
    Copyright (c) 2014-2017  Jon Lund Steffensen <jonlst@gmail.com>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::Provider;

use super::pipeutils::{
    pipeutils_create_nonblocking, pipeutils_handle_signal, pipeutils_signal,
};
use gio_sys::{
    g_bus_unwatch_name, g_bus_watch_name, g_dbus_error_get_remote_error,
    g_dbus_error_is_remote_error, g_dbus_proxy_call_sync,
    g_dbus_proxy_get_cached_property, g_dbus_proxy_get_connection,
    g_dbus_proxy_new_sync, GCancellable, GDBusConnection, GDBusInterfaceInfo,
    GDBusProxy, G_BUS_NAME_WATCHER_FLAGS_AUTO_START, G_BUS_TYPE_SYSTEM,
    G_DBUS_CALL_FLAGS_NONE, G_DBUS_PROXY_FLAGS_NONE,
};
use glib_sys::{
    g_error_free, g_free, g_io_channel_unix_new, g_io_channel_unref,
    g_io_create_watch, g_main_context_new, g_main_context_push_thread_default,
    g_main_context_unref, g_main_loop_new, g_main_loop_quit, g_main_loop_run,
    g_main_loop_unref, g_mutex_clear, g_mutex_init, g_mutex_lock,
    g_mutex_unlock, g_printerr, g_source_attach, g_source_set_callback,
    g_source_unref, g_strcmp0, g_thread_join, g_thread_new, g_variant_get,
    g_variant_get_child, g_variant_get_double, g_variant_new, g_variant_unref,
    gpointer, GError, GIOChannel, GIOCondition, GMainContext, GMainLoop,
    GMutex, GSource, GSourceFunc, GThread, GVariant, G_IO_ERR, G_IO_HUP,
    G_IO_IN,
};
use gobject_sys::{
    g_object_unref, g_signal_connect_data, GCallback, GObject,
    G_CONNECT_DEFAULT,
};
use libc::{close, fputs, free, malloc, FILE};
use std::ffi::{c_char, c_float, c_int, c_uint, c_void, CStr};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Geoclue2;
impl Provider for Geoclue2 {}

pub type gboolean = c_int;

struct location_t {
    lat: f32,
    lon: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct location_geoclue2_state_t {
    pub loop_0: *mut GMainLoop,
    pub thread: *mut GThread,
    pub lock: GMutex,
    pub pipe_fd_read: c_int,
    pub pipe_fd_write: c_int,
    pub available: c_int,
    pub error: c_int,
    pub latitude: c_float,
    pub longitude: c_float,
}

// Print the message explaining denial from GeoClue.
unsafe extern "C" fn print_denial_message() {
    g_printerr(
        // gettext(
            b"Access to the current location was denied by GeoClue!\nMake sure that location services are enabled and that Redshift is permitted\nto use location services. See https://github.com/jonls/redshift#faq for more\ninformation.\n\0"
                as *const u8 as *const c_char,
        // ),
    );
}

// Indicate an unrecoverable error during GeoClue2 communication.
unsafe extern "C" fn mark_error(state: *mut location_geoclue2_state_t) {
    g_mutex_lock(&mut (*state).lock);
    (*state).error = 1 as c_int;
    g_mutex_unlock(&mut (*state).lock);
    pipeutils_signal((*state).pipe_fd_write);
}

// Handle position change callbacks
unsafe extern "C" fn geoclue_client_signal_cb(
    client: *mut GDBusProxy,
    sender_name: *mut c_char,
    signal_name: *mut c_char,
    parameters: *mut GVariant,
    user_data: gpointer,
) {
    let state: *mut location_geoclue2_state_t =
        user_data as *mut location_geoclue2_state_t;

    // Only handle LocationUpdated signals
    if g_strcmp0(
        signal_name,
        b"LocationUpdated\0" as *const u8 as *const c_char,
    ) != 0 as c_int
    {
        return;
    }
    // Obtain location path
    let mut location_path: *const c_char = std::ptr::null::<c_char>();
    g_variant_get_child(
        parameters,
        1,
        b"&o\0" as *const u8 as *const c_char,
        &mut location_path as *mut *const c_char,
    );
    // Obtain location
    let mut error: *mut GError = std::ptr::null_mut::<GError>();
    let location: *mut GDBusProxy = g_dbus_proxy_new_sync(
        g_dbus_proxy_get_connection(client),
        G_DBUS_PROXY_FLAGS_NONE,
        std::ptr::null_mut::<GDBusInterfaceInfo>(),
        b"org.freedesktop.GeoClue2\0" as *const u8 as *const c_char,
        location_path,
        b"org.freedesktop.GeoClue2.Location\0" as *const u8 as *const c_char,
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );
    if location.is_null() {
        g_printerr(
            // gettext(
            b"Unable to obtain location: %s.\n\0" as *const u8 as *const c_char,
            // ),
            (*error).message,
        );
        g_error_free(error);
        mark_error(state);
        return;
    }
    g_mutex_lock(&mut (*state).lock);
    // Read location properties
    let lat_v: *mut GVariant = g_dbus_proxy_get_cached_property(
        location,
        b"Latitude\0" as *const u8 as *const c_char,
    );
    (*state).latitude = g_variant_get_double(lat_v) as c_float;
    let lon_v: *mut GVariant = g_dbus_proxy_get_cached_property(
        location,
        b"Longitude\0" as *const u8 as *const c_char,
    );
    (*state).longitude = g_variant_get_double(lon_v) as c_float;
    (*state).available = 1 as c_int;
    g_mutex_unlock(&mut (*state).lock);
    pipeutils_signal((*state).pipe_fd_write);
}

// Callback when GeoClue name appears on the bus
unsafe extern "C" fn on_name_appeared(
    conn: *mut GDBusConnection,
    name: *const c_char,
    name_owner: *const c_char,
    user_data: gpointer,
) {
    let state: *mut location_geoclue2_state_t =
        user_data as *mut location_geoclue2_state_t;
    // Obtain GeoClue Manager
    let mut error: *mut GError = std::ptr::null_mut::<GError>();
    let geoclue_manager: *mut GDBusProxy = g_dbus_proxy_new_sync(
        conn,
        G_DBUS_PROXY_FLAGS_NONE,
        std::ptr::null_mut::<GDBusInterfaceInfo>(),
        b"org.freedesktop.GeoClue2\0" as *const u8 as *const c_char,
        b"/org/freedesktop/GeoClue2/Manager\0" as *const u8 as *const c_char,
        b"org.freedesktop.GeoClue2.Manager\0" as *const u8 as *const c_char,
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );
    if geoclue_manager.is_null() {
        g_printerr(
            // gettext(
            b"Unable to obtain GeoClue Manager: %s.\n\0" as *const u8
                as *const c_char,
            // ),
            (*error).message,
        );
        g_error_free(error);
        mark_error(state);
        return;
    }
    // Obtain GeoClue Client path
    error = std::ptr::null_mut::<GError>();
    let client_path_v: *mut GVariant = g_dbus_proxy_call_sync(
        geoclue_manager,
        b"GetClient\0" as *const u8 as *const c_char,
        std::ptr::null_mut::<GVariant>(),
        G_DBUS_CALL_FLAGS_NONE,
        -(1 as c_int),
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );
    if client_path_v.is_null() {
        g_printerr(
            // gettext(
            b"Unable to obtain GeoClue client path: %s.\n\0" as *const u8
                as *const c_char,
            // ),
            (*error).message,
        );
        g_error_free(error);
        g_object_unref(geoclue_manager as *mut GObject);
        mark_error(state);
        return;
    }
    let mut client_path: *const c_char = std::ptr::null::<c_char>();
    g_variant_get(
        client_path_v,
        b"(&o)\0" as *const u8 as *const c_char,
        &mut client_path as *mut *const c_char,
    );
    // Obtain GeoClue client
    error = std::ptr::null_mut::<GError>();
    let geoclue_client: *mut GDBusProxy = g_dbus_proxy_new_sync(
        conn,
        G_DBUS_PROXY_FLAGS_NONE,
        std::ptr::null_mut::<GDBusInterfaceInfo>(),
        b"org.freedesktop.GeoClue2\0" as *const u8 as *const c_char,
        client_path,
        b"org.freedesktop.GeoClue2.Client\0" as *const u8 as *const c_char,
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );
    if geoclue_client.is_null() {
        g_printerr(
            // gettext(
            b"Unable to obtain GeoClue Client: %s.\n\0" as *const u8
                as *const c_char,
            // ),
            (*error).message,
        );
        g_error_free(error);
        g_variant_unref(client_path_v);
        g_object_unref(geoclue_manager as *mut GObject);
        mark_error(state);
        return;
    }
    g_variant_unref(client_path_v);

    // Set desktop id (basename of the .desktop file)
    error = std::ptr::null_mut::<GError>();
    let mut ret_v: *mut GVariant = g_dbus_proxy_call_sync(
        geoclue_client,
        b"org.freedesktop.DBus.Properties.Set\0" as *const u8 as *const c_char,
        g_variant_new(
            b"(ssv)\0" as *const u8 as *const c_char,
            b"org.freedesktop.GeoClue2.Client\0" as *const u8 as *const c_char,
            b"DesktopId\0" as *const u8 as *const c_char,
            g_variant_new(
                b"s\0" as *const u8 as *const c_char,
                b"redshift\0" as *const u8 as *const c_char,
            ),
        ),
        G_DBUS_CALL_FLAGS_NONE,
        -(1 as c_int),
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );

    // if (ret_v == NULL) {
    // // Ignore this error for now. The property is not available
    // // in early versions of GeoClue2.
    // } else {
    if !ret_v.is_null() {
        g_variant_unref(ret_v);
    }

    // Set distance threshold
    error = std::ptr::null_mut::<GError>();
    ret_v = g_dbus_proxy_call_sync(
        geoclue_client,
        b"org.freedesktop.DBus.Properties.Set\0" as *const u8 as *const c_char,
        g_variant_new(
            b"(ssv)\0" as *const u8 as *const c_char,
            b"org.freedesktop.GeoClue2.Client\0" as *const u8 as *const c_char,
            b"DistanceThreshold\0" as *const u8 as *const c_char,
            g_variant_new(b"u\0" as *const u8 as *const c_char, 50000 as c_int),
        ),
        G_DBUS_CALL_FLAGS_NONE,
        -(1 as c_int),
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );
    if ret_v.is_null() {
        g_printerr(
            // gettext(
            b"Unable to set distance threshold: %s.\n\0" as *const u8
                as *const c_char,
            // ),
            (*error).message,
        );
        g_error_free(error);
        g_object_unref(geoclue_client as *mut GObject);
        g_object_unref(geoclue_manager as *mut GObject);
        mark_error(state);
        return;
    }
    g_variant_unref(ret_v);

    // Attach signal callback to client
    g_signal_connect_data(
        geoclue_client as *mut GObject,
        b"g-signal\0" as *const u8 as *const c_char,
        ::core::mem::transmute::<
            Option<
                unsafe extern "C" fn(
                    *mut GDBusProxy,
                    *mut c_char,
                    *mut c_char,
                    *mut GVariant,
                    gpointer,
                ) -> (),
            >,
            GCallback,
        >(Some(
            geoclue_client_signal_cb
                as unsafe extern "C" fn(
                    *mut GDBusProxy,
                    *mut c_char,
                    *mut c_char,
                    *mut GVariant,
                    gpointer,
                ) -> (),
        )),
        user_data,
        None,
        G_CONNECT_DEFAULT,
    );

    // Start GeoClue client
    error = std::ptr::null_mut::<GError>();
    ret_v = g_dbus_proxy_call_sync(
        geoclue_client,
        b"Start\0" as *const u8 as *const c_char,
        std::ptr::null_mut::<GVariant>(),
        G_DBUS_CALL_FLAGS_NONE,
        -(1 as c_int),
        std::ptr::null_mut::<GCancellable>(),
        &mut error,
    );
    if ret_v.is_null() {
        g_printerr(
            // gettext(
            b"Unable to start GeoClue client: %s.\n\0" as *const u8
                as *const c_char,
            // ),
            (*error).message,
        );
        if g_dbus_error_is_remote_error(error) != 0 {
            let dbus_error: *mut c_char = g_dbus_error_get_remote_error(error);
            if g_strcmp0(
                dbus_error,
                b"org.freedesktop.DBus.Error.AccessDenied\0" as *const u8
                    as *const c_char,
            ) == 0 as c_int
            {
                print_denial_message();
            }
            g_free(dbus_error as gpointer);
        }
        g_error_free(error);
        g_object_unref(geoclue_client as *mut GObject);
        g_object_unref(geoclue_manager as *mut GObject);
        mark_error(state);
        return;
    }
    g_variant_unref(ret_v);
}

// Callback when GeoClue disappears from the bus
unsafe extern "C" fn on_name_vanished(
    connection: *mut GDBusConnection,
    name: *const c_char,
    user_data: gpointer,
) {
    let state: *mut location_geoclue2_state_t =
        user_data as *mut location_geoclue2_state_t;
    g_mutex_lock(&mut (*state).lock);
    (*state).available = 0 as c_int;
    g_mutex_unlock(&mut (*state).lock);
    pipeutils_signal((*state).pipe_fd_write);
}

// Callback when the pipe to the main thread is closed.
unsafe extern "C" fn on_pipe_closed(
    channel: *mut GIOChannel,
    condition: GIOCondition,
    user_data: gpointer,
) -> gboolean {
    let state: *mut location_geoclue2_state_t =
        user_data as *mut location_geoclue2_state_t;
    g_main_loop_quit((*state).loop_0);
    0 as c_int
}

// Run loop for location provider thread.
unsafe extern "C" fn run_geoclue2_loop(state_: *mut c_void) -> *mut c_void {
    let state: *mut location_geoclue2_state_t =
        state_ as *mut location_geoclue2_state_t;
    let context: *mut GMainContext = g_main_context_new();
    g_main_context_push_thread_default(context);
    (*state).loop_0 = g_main_loop_new(context, 0 as c_int);
    let watcher_id: c_uint = g_bus_watch_name(
        G_BUS_TYPE_SYSTEM,
        b"org.freedesktop.GeoClue2\0" as *const u8 as *const c_char,
        G_BUS_NAME_WATCHER_FLAGS_AUTO_START,
        Some(
            on_name_appeared
                as unsafe extern "C" fn(
                    *mut GDBusConnection,
                    *const c_char,
                    *const c_char,
                    gpointer,
                ) -> (),
        ),
        Some(
            on_name_vanished
                as unsafe extern "C" fn(
                    *mut GDBusConnection,
                    *const c_char,
                    gpointer,
                ) -> (),
        ),
        state as gpointer,
        None,
    );

    // Listen for closure of pipe
    let pipe_channel: *mut GIOChannel =
        g_io_channel_unix_new((*state).pipe_fd_write);
    let pipe_source: *mut GSource = g_io_create_watch(
        pipe_channel,
        (G_IO_IN as c_int | G_IO_HUP as c_int | G_IO_ERR as c_int)
            as GIOCondition,
    );
    g_source_set_callback(
        pipe_source,
        ::core::mem::transmute::<
            Option<
                unsafe extern "C" fn(
                    *mut GIOChannel,
                    GIOCondition,
                    gpointer,
                ) -> gboolean,
            >,
            GSourceFunc,
        >(Some(
            on_pipe_closed
                as unsafe extern "C" fn(
                    *mut GIOChannel,
                    GIOCondition,
                    gpointer,
                ) -> gboolean,
        )),
        state as gpointer,
        None,
    );
    g_source_attach(pipe_source, context);
    g_main_loop_run((*state).loop_0);
    g_source_unref(pipe_source);
    g_io_channel_unref(pipe_channel);
    close((*state).pipe_fd_write);
    g_bus_unwatch_name(watcher_id);
    g_main_loop_unref((*state).loop_0);
    g_main_context_unref(context);
    std::ptr::null_mut::<c_void>()
}
unsafe extern "C" fn location_geoclue2_init(
    state: *mut *mut location_geoclue2_state_t,
) -> c_int {
    *state = malloc(::core::mem::size_of::<location_geoclue2_state_t>())
        as *mut location_geoclue2_state_t;
    if (*state).is_null() {
        return -(1 as c_int);
    }
    0 as c_int
}
unsafe extern "C" fn location_geoclue2_start(
    state: *mut location_geoclue2_state_t,
) -> c_int {
    (*state).pipe_fd_read = -(1 as c_int);
    (*state).pipe_fd_write = -(1 as c_int);
    (*state).available = 0 as c_int;
    (*state).error = 0 as c_int;
    (*state).latitude = 0 as c_int as c_float;
    (*state).longitude = 0 as c_int as c_float;
    let mut pipefds: [c_int; 2] = [0; 2];
    let r: c_int = pipeutils_create_nonblocking(pipefds.as_mut_ptr());
    if r < 0 as c_int {
        // gettext(
        eprintln!("Failed to start GeoClue2 provider!");
        return -(1 as c_int);
    }
    (*state).pipe_fd_read = pipefds[0 as c_int as usize];
    (*state).pipe_fd_write = pipefds[1 as c_int as usize];
    pipeutils_signal((*state).pipe_fd_write);
    g_mutex_init(&mut (*state).lock);
    (*state).thread = g_thread_new(
        b"geoclue2\0" as *const u8 as *const c_char,
        Some(
            run_geoclue2_loop
                as unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        ),
        state as gpointer,
    );
    0 as c_int
}
unsafe extern "C" fn location_geoclue2_free(
    state: *mut location_geoclue2_state_t,
) {
    if (*state).pipe_fd_read != -(1 as c_int) {
        close((*state).pipe_fd_read);
    }
    // Closing the pipe should cause the thread to exit.
    g_thread_join((*state).thread);
    (*state).thread = std::ptr::null_mut::<GThread>();
    g_mutex_clear(&mut (*state).lock);
    free(state as *mut c_void);
}
unsafe extern "C" fn location_geoclue2_print_help(f: *mut FILE) {
    fputs(
        // gettext(
        b"Use the location as discovered by a GeoClue2 provider.\n\0"
            as *const u8 as *const c_char,
        // ),
        f,
    );
    fputs(b"\n\0" as *const u8 as *const c_char, f);
}
unsafe extern "C" fn location_geoclue2_set_option(
    state: *mut location_geoclue2_state_t,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    // gettext(
    eprintln!(
        "Unknown method parameter: `{}`.",
        CStr::from_ptr(key).to_str().unwrap()
    );
    -(1 as c_int)
}
unsafe extern "C" fn location_geoclue2_get_fd(
    state: *mut location_geoclue2_state_t,
) -> c_int {
    (*state).pipe_fd_read
}
unsafe extern "C" fn location_geoclue2_handle(
    state: *mut location_geoclue2_state_t,
    location: *mut location_t,
    available: *mut c_int,
) -> c_int {
    pipeutils_handle_signal((*state).pipe_fd_read);
    g_mutex_lock(&mut (*state).lock);
    let error: c_int = (*state).error;
    (*location).lat = (*state).latitude;
    (*location).lon = (*state).longitude;
    *available = (*state).available;
    g_mutex_unlock(&mut (*state).lock);
    if error != 0 {
        return -(1 as c_int);
    }
    0 as c_int
}
