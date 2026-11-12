import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.Lua.LuaEventManager;
import zombie.core.random.RandStandard;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
public class FishingEventProbe {
    static void run(String code, String name) throws Exception {
        Object[] result=LuaManager.thread.pcall(LuaCompiler.loadstring(code,name,LuaManager.env),new Object[0]);
        if(!Boolean.TRUE.equals(result[0])) throw new IllegalStateException(Arrays.toString(result));
    }
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-event-probe"); ZomboidFileSystem.instance.init();
        RandStandard.INSTANCE.init(); LuaManager.init(); LuaEventManager.AddEvent("EveryDays");
        run("updates=0; local manager={}; function manager:updateSeed() updates=updates+1 end; FishSchoolManager={getInstance=function() return manager end}","stub");
        run(Files.readString(Path.of(args[0])),"FishingUtils.lua");
        System.out.println("FISH before="+LuaManager.env.rawget("updates"));
        LuaEventManager.triggerEvent("EveryHours"); System.out.println("FISH hour="+LuaManager.env.rawget("updates"));
        LuaEventManager.triggerEvent("EveryDays"); System.out.println("FISH day1="+LuaManager.env.rawget("updates"));
        LuaEventManager.triggerEvent("EveryDays"); System.out.println("FISH day2="+LuaManager.env.rawget("updates"));
    }
}
