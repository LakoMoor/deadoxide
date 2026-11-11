import java.io.*;
import java.lang.reflect.*;
import zombie.GameTime;
import zombie.core.random.RandStandard;
import zombie.core.PerformanceSettings;
import zombie.Lua.LuaManager;
import zombie.ZomboidFileSystem;

public class ClockBoundaryProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init(); PerformanceSettings.setLockFPS(30);
        Method stamp = GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true);
        Field prev = GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true);
        Field mod = GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true);
        int[][] dates = {{1993,6,8},{1993,0,30},{1993,11,30},{1996,1,27},{1993,1,27},{2000,1,27},{1900,1,27}};
        for (int[] date : dates) {
            GameTime t = new GameTime(); GameTime.setInstance(t);
            try (var in = new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))) { t.load(in); }
            t.setYear(date[0]); t.setMonth(date[1]); t.setDay(date[2]);
            t.setTimeOfDay(23.99998f); t.setLastTimeOfDay(23.99998f); t.lastLastTimeOfDay=23.99998f;
            t.setMinutesPerDay(90f); t.setMultiplier(1f);
            stamp.invoke(t); prev.setLong(t,t.getMinutesStamp()); mod.setInt(t,0);
            try {
                t.update(false);
                System.out.printf("DATE %d,%d,%d=%08x,%d,%d,%d%n", date[0], date[1], date[2], Float.floatToRawIntBits(t.getTimeOfDay()), t.getYear(), t.getMonth(), t.getDay());
            } catch (Throwable e) { System.out.println("ERROR " + e); e.printStackTrace(); }
        }
    }
}
