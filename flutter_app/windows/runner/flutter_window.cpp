#include "flutter_window.h"

#include <optional>
#include <string>

#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <flutter/encodable_value.h>
#include "flutter/generated_plugin_registrant.h"
#include "auto_start.h"

FlutterWindow::FlutterWindow(const flutter::DartProject& project)
    : project_(project) {}

FlutterWindow::~FlutterWindow() {}

bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  RECT frame = GetClientArea();

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  // Ensure that basic setup of the controller was successful.
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  RegisterPlugins(flutter_controller_->engine());
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  // Register method channel for auto-start
  auto messenger = flutter_controller_->engine()->messenger();
  const std::string channel_name = "com.toss/auto_start";
  auto channel = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      messenger, channel_name, &flutter::StandardMethodCodec::GetInstance());
  
  // Keep channel alive by storing in a static variable
  static std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>> channel_storage;
  channel_storage = std::move(channel);

  channel_storage->SetMethodCallHandler(
      [](const flutter::MethodCall<flutter::EncodableValue>& call,
         std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
        if (call.method_name() == "enableAutoStart") {
          auto* args = std::get_if<flutter::EncodableMap>(call.arguments());
          if (args) {
            auto app_path_it = args->find(flutter::EncodableValue("appPath"));
            if (app_path_it != args->end()) {
              auto app_path = std::get<std::string>(app_path_it->second);
              bool success = EnableAutoStart(app_path.c_str());
              result->Success(flutter::EncodableValue(success));
            } else {
              result->Error("INVALID_ARGUMENT", "appPath is required");
            }
          } else {
            result->Error("INVALID_ARGUMENT", "Arguments must be a map");
          }
        } else if (call.method_name() == "disableAutoStart") {
          bool success = DisableAutoStart();
          result->Success(flutter::EncodableValue(success));
        } else if (call.method_name() == "isAutoStartEnabled") {
          bool enabled = IsAutoStartEnabled();
          result->Success(flutter::EncodableValue(enabled));
        } else {
          result->NotImplemented();
        }
      });

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    this->Show();
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::OnDestroy() {
  if (flutter_controller_) {
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  // Give Flutter, including plugins, an opportunity to handle window messages.
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }
  }

  switch (message) {
    case WM_FONTCHANGE:
      flutter_controller_->engine()->ReloadSystemFonts();
      break;
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}
