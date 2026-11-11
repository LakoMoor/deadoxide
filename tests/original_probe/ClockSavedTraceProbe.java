import java.io.*;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import zombie.GameTime;
import zombie.SandboxOptions;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.core.PerformanceSettings;
import zombie.core.random.RandStandard;
public class ClockSavedTraceProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init(); PerformanceSettings.setLockFPS(30);
        SandboxOptions.instance.load(ByteBuffer.wrap(Files.readAllBytes(Path.of(args[1]))));
        GameTime t=new GameTime(); GameTime.setInstance(t);
        try(var in=new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))){t.load(in);}
        t.setMinutesPerDay(SandboxOptions.instance.getDayLengthMinutes()); t.setMultiplier(1f); t.fpsMultiplier=2f;
        t.setLastTimeOfDay(t.getTimeOfDay()); t.lastLastTimeOfDay=t.getTimeOfDay();
        var stamp=GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true); stamp.invoke(t);
        var prev=GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true); prev.setLong(t,t.getMinutesStamp());
        var mod=GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true); mod.setInt(t,t.getMinutes()/10);
        for(int i=0;i<100;i++) t.update(false);
        System.out.printf("SAVE ticks=100 bits=%08x year=%d month_index=%d day_index=%d%n",Float.floatToRawIntBits(t.getTimeOfDay()),t.getYear(),t.getMonth(),t.getDay());
    }
}
