import java.io.*;
import java.lang.reflect.*;
import zombie.GameTime;
import zombie.core.random.RandStandard;
import zombie.core.PerformanceSettings;
import zombie.Lua.LuaManager;
import zombie.ZomboidFileSystem;

public class ClockFixed30Probe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init(); PerformanceSettings.setLockFPS(30);
        var stamp = GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true);
        var prev = GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true);
        var mod = GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true);
        for (float perDay : new float[]{30f,60f,90f}) {
            var t = new GameTime(); GameTime.setInstance(t);
            try(var in=new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))){t.load(in);}
            t.setTimeOfDay(11.42f); t.setLastTimeOfDay(11.42f); t.lastLastTimeOfDay=11.42f;
            t.setMinutesPerDay(perDay); t.setMultiplier(1f); t.fpsMultiplier=2f;
            stamp.invoke(t); prev.setLong(t,t.getMinutesStamp()); mod.setInt(t,t.getMinutes()/10);
            System.out.printf("CASE %.0f actual=%08x start=%08x",perDay,Float.floatToRawIntBits(t.getMultiplier()),Float.floatToRawIntBits(t.getTimeOfDay()));
            for(int i=1;i<=100;i++){t.update(false);if(i==1||i==30||i==100)System.out.printf(",%08x",Float.floatToRawIntBits(t.getTimeOfDay()));}
            System.out.println();
        }
    }
}
