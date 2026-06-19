.PHONY: dev-gui run-gui test

# 监听变更并自动重新运行 GUI（推荐日常开发）
dev-gui:
	./scripts/dev-gui.sh

# 单次运行 GUI
run-gui:
	cargo run -p egui-app

test:
	cargo test --workspace
