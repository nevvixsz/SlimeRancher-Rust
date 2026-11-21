using System;
using System.IO;
using System.Text;

namespace SlimeCoop
{
    internal static class CoopLog
    {
        private static readonly object LogLock = new object();
        private static string _logPath;
        private static bool _initialized;

        public static string LogFilePath => _logPath ?? string.Empty;

        public static void Initialize(string persistentDataPath)
        {
            if (_initialized) return;
            try
            {
                string dir = Path.Combine(persistentDataPath, "SRML", "Config", "slimecoop");
                Directory.CreateDirectory(dir);
                _logPath = Path.Combine(dir, "slimecoop.log");

                if (File.Exists(_logPath) && new FileInfo(_logPath).Length > 512 * 1024)
                {
                    string prev = Path.Combine(dir, "slimecoop.prev.log");
                    if (File.Exists(prev)) File.Delete(prev);
                    File.Move(_logPath, prev);
                }

                _initialized = true;
                Write("INFO", "=== SlimeCoop v1.0.0 started (" + DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss") + ") ===");
            }
            catch
            {
            }
        }

        public static void Info(string msg)
        {
            Write("INFO", msg);
        }

        public static void Warn(string msg)
        {
            Write("WARN", msg);
        }

        public static void Error(string msg)
        {
            Write("ERROR", msg);
        }

        private static void Write(string level, string msg)
        {
            if (string.IsNullOrEmpty(_logPath)) return;
            try
            {
                string line = string.Format("[{0}] [{1}] {2}{3}",
                    DateTime.Now.ToString("HH:mm:ss.fff"), level, msg ?? string.Empty, Environment.NewLine);
                lock (LogLock)
                {
                    File.AppendAllText(_logPath, line, Encoding.UTF8);
                }
            }
            catch
            {
            }
        }
    }
}
