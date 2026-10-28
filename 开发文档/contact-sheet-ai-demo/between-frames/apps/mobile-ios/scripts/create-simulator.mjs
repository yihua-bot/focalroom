// 创建独立评审设备，保留用户现有模拟器与数据。
import { execFileSync } from 'node:child_process';
const xcrun = args => execFileSync('xcrun', ['simctl', ...args], { encoding: 'utf8' });
const runtime = JSON.parse(xcrun(['list', 'runtimes', '-j'])).runtimes
  .filter(item => item.isAvailable && item.identifier.includes('.iOS-'))
  .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
if (!runtime) throw new Error('请先在 Xcode 安装可用的 iOS Simulator runtime');
const device = runtime.supportedDeviceTypes?.find(item => item.productFamily === 'iPhone');
if (!device) throw new Error('当前 runtime 没有报告支持的 iPhone 设备类型');
process.stdout.write(xcrun(['create', 'FocalRoom iPhone Review', device.identifier, runtime.identifier]).trim() + '\n');
