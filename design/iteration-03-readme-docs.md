# 第三期迭代：开源 README 与项目展示文档

## 文档状态

本文档记录 `tracelens` 第三期文档迭代的范围、实施内容和验收标准。

本期不是功能迭代，不新增 CLI 分析能力。它补齐开源项目首页文档，让用户进入仓库后能够快速理解项目定位、当前能力、安装方式、使用方式和路线图。

## 本期目标

本期目标是让项目具备基本开源展示能力：

- 默认英文 README。
- 中文 README。
- README 顶部 logo。
- 清晰说明当前已支持能力和未完成能力。
- 链接到里程碑和进度文档。

## 本期交付物

- `README.md`：默认英文 README。
- `README.zh-CN.md`：中文 README。
- `assets/logo.svg`：项目 logo。
- 更新 `design/progress.md`，记录本次文档补充对进度的影响。
- 更新 `design/milestones.md`，明确 README 属于发布与分发里程碑中的文档基础能力。

## 本期不做

本期明确不做：

- 不新增 CLI 命令。
- 不修改 trace 分析逻辑。
- 不发布 release artifact。
- 不生成二进制 checksum。
- 不实现远程安装脚本。
- 不实现 HTML report。

## 验收标准

本期完成时应满足：

- GitHub 默认展示 `README.md` 英文版。
- 英文 README 能链接到中文 README。
- 中文 README 能链接回英文 README。
- README 不夸大当前能力。
- README 明确说明 release artifact 尚未发布。
- README 中的命令与当前 CLI 能力一致。
- Logo 使用本地 SVG，不依赖远程图片。
- `design/progress.md` 已更新。
- `design/milestones.md` 已更新。

## 与里程碑的对应关系

| 里程碑 | 本期覆盖情况 |
| --- | --- |
| M0：范围与工程骨架 | 补充项目首页文档 |
| M9：发布与分发 | 补充安装说明、使用示例和开源展示基础 |

## 实施说明

本期 README 只描述当前已经实现的能力，把关键路径、N+1、HTML report、远程 release artifact 等未完成能力明确放在项目状态和路线图中。
