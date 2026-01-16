#include <windows.h>
#include <string>

// Enable auto-start by adding registry entry
extern "C" __declspec(dllexport) bool EnableAutoStart(const char* appPath) {
    HKEY hKey;
    LONG result = RegOpenKeyExA(
        HKEY_CURRENT_USER,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        0,
        KEY_SET_VALUE,
        &hKey
    );

    if (result != ERROR_SUCCESS) {
        return false;
    }

    std::string value = std::string("\"") + appPath + std::string("\"");
    result = RegSetValueExA(
        hKey,
        "Toss",
        0,
        REG_SZ,
        (const BYTE*)value.c_str(),
        static_cast<DWORD>(value.length() + 1)
    );

    RegCloseKey(hKey);
    return result == ERROR_SUCCESS;
}

// Disable auto-start by removing registry entry
extern "C" __declspec(dllexport) bool DisableAutoStart() {
    HKEY hKey;
    LONG result = RegOpenKeyExA(
        HKEY_CURRENT_USER,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        0,
        KEY_SET_VALUE,
        &hKey
    );

    if (result != ERROR_SUCCESS) {
        return false;
    }

    result = RegDeleteValueA(hKey, "Toss");
    RegCloseKey(hKey);
    return result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND;
}

// Check if auto-start is enabled
extern "C" __declspec(dllexport) bool IsAutoStartEnabled() {
    HKEY hKey;
    LONG result = RegOpenKeyExA(
        HKEY_CURRENT_USER,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        0,
        KEY_QUERY_VALUE,
        &hKey
    );

    if (result != ERROR_SUCCESS) {
        return false;
    }

    DWORD type;
    DWORD size = 0;
    result = RegQueryValueExA(hKey, "Toss", NULL, &type, NULL, &size);
    RegCloseKey(hKey);

    return result == ERROR_SUCCESS && type == REG_SZ;
}
