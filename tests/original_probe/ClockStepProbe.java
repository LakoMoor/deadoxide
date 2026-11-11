import java.io.*;
import java.lang.reflect.*;
import zombie.GameTime;
import zombie.core.random.RandStandard;
import zombie.core.PerformanceSettings;
import zombie.Lua.LuaManager;
import zombie.ZomboidFileSystem;

public class ClockStepProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe");
        ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init();
        LuaManager.init();
        PerformanceSettings.setLockFPS(30);
        Method stamp = GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true);
        Field prev = GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true);
        Field mod = GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true);
        for (float perDay : new float[]{30f, 60f, 90f}) {
            for (float speed : new float[]{0.0f, 0.8f, 1.0f, 2.0f}) {
                GameTime time = new GameTime(); GameTime.setInstance(time);
                try (var in = new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))) { time.load(in); }
                time.setTimeOfDay(11.42f); time.setLastTimeOfDay(11.42f); time.lastLastTimeOfDay=11.42f;
                time.setMinutesPerDay(perDay); time.setMultiplier(speed);
                stamp.invoke(time); prev.setLong(time, time.getMinutesStamp()); mod.setInt(time, time.getMinutes()/10);
                System.out.printf("CASE %.0f %.1f actual=%08x,%08x start=%08x", perDay, speed,
                    Float.floatToRawIntBits(time.getMinutesPerDay()), Float.floatToRawIntBits(time.getMultiplier()), Float.floatToRawIntBits(time.getTimeOfDay()));
                for(int i=1;i<=100;i++) { time.update(false); if(i==1||i==30||i==100) System.out.printf(",%08x", Float.floatToRawIntBits(time.getTimeOfDay())); }
                System.out.println();
            }
        }
    }
}
