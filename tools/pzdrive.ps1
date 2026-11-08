param([string]$Action, [string]$Arg = "", [string]$Arg2 = "")
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class W {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@
$p = Get-Process -Id $env:PZ_PID -ErrorAction Stop
$h = $p.MainWindowHandle
function Focus { [W]::ShowWindow($h, 9) | Out-Null; [W]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 400 }
function ClientOrigin { $pt = New-Object W+POINT; [W]::ClientToScreen($h, [ref]$pt) | Out-Null; $r = New-Object W+RECT; [W]::GetClientRect($h, [ref]$r) | Out-Null; return @($pt.X, $pt.Y, $r.R, $r.B) }
switch ($Action) {
  "key"   { Focus; $vk = [Convert]::ToByte($Arg, 16); [W]::keybd_event($vk, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80; [W]::keybd_event($vk, 0, 2, [UIntPtr]::Zero) }
  "hold"  { Focus; $vk = [Convert]::ToByte($Arg, 16); [W]::keybd_event($vk, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds ([int]$Arg2); [W]::keybd_event($vk, 0, 2, [UIntPtr]::Zero) }
  "click" { Focus; $o = ClientOrigin; $x = $o[0] + [int]$Arg; $y = $o[1] + [int]$Arg2; [W]::SetCursorPos($x, $y) | Out-Null; Start-Sleep -Milliseconds 100; [W]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60; [W]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero) }
  "shot"  { Focus; $o = ClientOrigin; $bmp = New-Object System.Drawing.Bitmap $o[2], $o[3]; $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($o[0], $o[1], 0, 0, $bmp.Size); $bmp.Save($Arg); "client area $($o[2])x$($o[3]) at $($o[0]),$($o[1]) -> $Arg" }
  "wheel" { Focus; $o = ClientOrigin; [W]::SetCursorPos($o[0] + [int]$Arg, $o[1] + [int]$Arg2) | Out-Null; Start-Sleep -Milliseconds 150; 1..3 | ForEach-Object { [W]::mouse_event(0x800, 0, 0, [uint32]4294967176, [UIntPtr]::Zero); Start-Sleep -Milliseconds 150 } }
  "rect"  { $o = ClientOrigin; "client $($o[2])x$($o[3]) at $($o[0]),$($o[1])" }
}
