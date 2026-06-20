.PHONY: dev-gui dev-gui-watch run-gui run-gui-macos test package-dmg

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
	./scripts/run-gui-macos.sh

test:
	cargo test --workspace

# macOS：Release .app + DMG → dist/SwitchHostsRust.dmg
package-dmg:
	./scripts/package-macos.sh --dmg
