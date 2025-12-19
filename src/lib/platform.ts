export const platform = {
  isMac: navigator.userAgent.toLowerCase().includes("mac"),
  isWindows: navigator.userAgent.toLowerCase().includes("win"),
  isLinux: navigator.userAgent.toLowerCase().includes("linux"),
};
