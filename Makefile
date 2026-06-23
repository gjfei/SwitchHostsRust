.PHONY: dev-gui dev-gui-watch run-gui run-gui-macos test package-dmg package-gui sync-fonts sync-icons

# 单次运行 GUI；退出后终端回到 shell（推荐日常调试）
dev-gui:
	cargo dev-gui

# 监听变更并自动重新运行（需 Ctrl+C 结束 watch 进程）
dev-gui-watch:
	cargo dev-gui-watch

# 单次运行 GUI（macOS 请用 run-gui-macos 以显示 Mission Control 图标）
run-gui:
	cargo run -p egui-app

# macOS：通过 .app 启动（Mission Control / Dock 图标完整）
run-gui-macos:
	cargo run-gui-macos

test:
	cargo test --workspace

# macOS：Release DMG → dist/
package-dmg:
	cargo package-dmg

# 仅 GUI
package-gui:
	cargo package-macos -- --app egui-app

sync-fonts:
	cargo sync-fonts

sync-icons:
	cargo sync-icons
