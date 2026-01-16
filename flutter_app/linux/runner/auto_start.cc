#include "auto_start.h"

#include <glib.h>
#include <glib/gstdio.h>
#include <string.h>

// Get the path to the autostart directory
static gchar* get_autostart_dir(void) {
  const gchar* config_home = g_getenv("XDG_CONFIG_HOME");
  if (config_home != nullptr) {
    return g_build_filename(config_home, "autostart", nullptr);
  }
  
  const gchar* home = g_getenv("HOME");
  if (home != nullptr) {
    return g_build_filename(home, ".config", "autostart", nullptr);
  }
  
  return nullptr;
}

// Get the path to the .desktop file
static gchar* get_desktop_file_path(void) {
  gchar* autostart_dir = get_autostart_dir();
  if (autostart_dir == nullptr) {
    return nullptr;
  }
  
  // Create autostart directory if it doesn't exist
  g_mkdir_with_parents(autostart_dir, 0755);
  
  gchar* desktop_file = g_build_filename(autostart_dir, "toss.desktop", nullptr);
  g_free(autostart_dir);
  
  return desktop_file;
}

// Get the executable path
static gchar* get_executable_path(void) {
  gchar* exe_path = g_file_read_link("/proc/self/exe", nullptr);
  if (exe_path == nullptr) {
    // Fallback: try to find the executable in PATH
    exe_path = g_find_program_in_path("toss");
  }
  return exe_path;
}

gboolean auto_start_set_enabled(gboolean enabled) {
  if (enabled) {
    // Create .desktop file
    gchar* desktop_file = get_desktop_file_path();
    if (desktop_file == nullptr) {
      return FALSE;
    }
    
    gchar* exe_path = get_executable_path();
    if (exe_path == nullptr) {
      g_free(desktop_file);
      return FALSE;
    }
    
    GString* content = g_string_new(nullptr);
    g_string_append_printf(content, "[Desktop Entry]\n");
    g_string_append_printf(content, "Type=Application\n");
    g_string_append_printf(content, "Name=Toss\n");
    g_string_append_printf(content, "Exec=%s\n", exe_path);
    g_string_append_printf(content, "Terminal=false\n");
    g_string_append_printf(content, "NoDisplay=false\n");
    g_string_append_printf(content, "Hidden=false\n");
    g_string_append_printf(content, "X-GNOME-Autostart-enabled=true\n");
    
    GError* error = nullptr;
    gboolean success = g_file_set_contents(
        desktop_file, content->str, -1, &error);
    
    if (!success && error != nullptr) {
      g_warning("Failed to write desktop file: %s", error->message);
      g_error_free(error);
    }
    
    g_string_free(content, TRUE);
    g_free(exe_path);
    g_free(desktop_file);
    
    return success;
  } else {
    // Remove .desktop file
    gchar* desktop_file = get_desktop_file_path();
    if (desktop_file == nullptr) {
      return FALSE;
    }
    
    gboolean success = TRUE;
    if (g_file_test(desktop_file, G_FILE_TEST_EXISTS)) {
      if (g_unlink(desktop_file) != 0) {
        g_warning("Failed to delete desktop file: %s", desktop_file);
        success = FALSE;
      }
    }
    
    g_free(desktop_file);
    return success;
  }
}

gboolean auto_start_is_enabled(void) {
  gchar* desktop_file = get_desktop_file_path();
  if (desktop_file == nullptr) {
    return FALSE;
  }
  
  gboolean exists = g_file_test(desktop_file, G_FILE_TEST_EXISTS);
  g_free(desktop_file);
  
  return exists;
}
