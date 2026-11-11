# volanik #

一些好看的程序化生成的内容。

- `main` - 程序入口
- `window` - winit 窗口管理和输入管理
- `render` - GPU 资源管理
- `camera` - 场景绑定和相机控制
- `skybox` - 体积云和天空盒
- `debug` - egui 调试面板
- `shaders` 着色器

# 操作 #

WASD 前后左右 RF 上升下降 ZX 时间控制 Shift 加速 F1 显示/隐藏调试面板

# 调试 / 导出 #

- `VOLANIK_EXPORT_PATH=out.png` - 渲染 3 帧后导出 PNG（默认继续运行；配合 `VOLANIK_EXPORT_EXIT=1` 导出后退出）
- `VOLANIK_SIZE=1280x720` - 覆盖窗口尺寸
- `VOLANIK_TIME=21600` - 固定起始时刻，单位秒
- `VOLANIK_CAM_POS=x,y,z` / `VOLANIK_CAM_ANGLES=yaw,pitch,roll` - 固定相机位置和朝向（角度制）
- `VOLANIK_HIDE_UI=1` - 导出时隐藏调试面板
- `VOLANIK_CLOUD_COVERAGE`、`VOLANIK_CLOUD_DENSITY`、`VOLANIK_CLOUD_QUALITY` - 覆盖云参数

# 坐标系 & 常量 #

右手系，+z 为上，+y 为北，弧度优先。

z0 作为地表
