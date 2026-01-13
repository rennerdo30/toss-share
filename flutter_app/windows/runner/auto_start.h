#ifndef RUNNER_AUTO_START_H_
#define RUNNER_AUTO_START_H_

#ifdef __cplusplus
extern "C" {
#endif

// Enable auto-start by adding registry entry
bool EnableAutoStart(const char* appPath);

// Disable auto-start by removing registry entry
bool DisableAutoStart();

// Check if auto-start is enabled
bool IsAutoStartEnabled();

#ifdef __cplusplus
}
#endif

#endif  // RUNNER_AUTO_START_H_
