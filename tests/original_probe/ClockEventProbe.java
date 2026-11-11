import java.io.*;
import java.lang.reflect.*;
import java.util.*;
import zombie.GameTime;
import zombie.core.random.RandStandard;
import zombie.core.PerformanceSettings;
import zombie.Lua.LuaManager;
import zombie.Lua.LuaEventManager;
import zombie.ZomboidFileSystem;
import zombie.erosion.ErosionMain;
import zombie.iso.sprite.IsoSpriteManager;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import se.krka.kahlua.vm.JavaFunction;

public class ClockEventProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-time-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init(); PerformanceSettings.setLockFPS(30);
        new ErosionMain(IsoSpriteManager.instance, false);
        var trace = new ArrayList<String>();
        LuaManager.env.rawset("recordClockEvent", (JavaFunction)(frame, n) -> { trace.add((String)frame.get(0)); return 0; });
        for (String name : new String[]{"EveryOneMinute", "EveryTenMinutes", "EveryHours", "EveryDays"}) {
            var event = LuaEventManager.AddEvent(name);
            event.callbacks.add(LuaCompiler.loadstring("recordClockEvent('"+name+"')", "clock-event-probe", LuaManager.env));
        }
        Method stamp = GameTime.class.getDeclaredMethod("setMinutesStamp"); stamp.setAccessible(true);
        Field prev = GameTime.class.getDeclaredField("previousMinuteStamp"); prev.setAccessible(true);
        Field mod = GameTime.class.getDeclaredField("minutesMod"); mod.setAccessible(true);
        for (String spec : args[1].split(",")) {
            String[] parts = spec.split("@");
            float start = Float.parseFloat(parts[0]);
            int nights = Integer.parseInt(parts[1]);
            GameTime t=new GameTime(); GameTime.setInstance(t);
            try(var in=new DataInputStream(new BufferedInputStream(new FileInputStream(args[0])))){t.load(in);}
            t.setNightsSurvived(nights);
            t.setTimeOfDay(start); t.setLastTimeOfDay(start); t.lastLastTimeOfDay=start;
            t.setMinutesPerDay(90f); t.setMultiplier(1f); t.fpsMultiplier=2f;
            stamp.invoke(t); prev.setLong(t,t.getMinutesStamp()); mod.setInt(t,t.getMinutes()/10);
            trace.clear();
            try {
                t.update(false);
                System.out.printf("EVENT %s=%08x,%d,%s%n",spec,Float.floatToRawIntBits(t.getTimeOfDay()),t.getNightsSurvived(),String.join("|",trace));
            } catch(Throwable ex) { System.out.println("ERROR "+spec+" "+ex); ex.printStackTrace(); }
        }
    }
}
