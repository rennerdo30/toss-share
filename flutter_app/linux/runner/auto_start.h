#ifndef RUNNER_AUTO_START_H_
#define RUNNER_AUTO_START_H_

#include <glib.h>

#ifdef __cplusplus
extern "C" {
#endif

// Sets or unsets the application to start automatically with the system.
// Returns TRUE on success, FALSE on failure.
gboolean auto_start_set_enabled(gboolean enabled);

// Checks if the application is configured to start automatically.
// Returns TRUE if enabled, FALSE otherwise.
gboolean auto_start_is_enabled(void);

#ifdef __cplusplus
}
#endif

#endif  // RUNNER_AUTO_START_H_
