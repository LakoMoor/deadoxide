import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.Lua.LuaEventManager;
import zombie.core.random.RandStandard;
import zombie.core.random.RandAbstract;
import zombie.iso.FishSchoolManager;
import org.uncommons.maths.random.CellularAutomatonRNG;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
public class FishingDailySeedProbe {
  static void run(String source,String name) throws Exception {
    Object[] result=LuaManager.thread.pcall(LuaCompiler.loadstring(source,name,LuaManager.env),new Object[0]);
    if(!Boolean.TRUE.equals(result[0])) throw new IllegalStateException(Arrays.toString(result));
  }
  public static void main(String[] args) throws Exception {
    ZomboidFileSystem.instance.setCacheDir("/tmp/pz-fishing-daily-seed"); ZomboidFileSystem.instance.init(); RandStandard.INSTANCE.init(); LuaManager.init(); LuaEventManager.AddEvent("EveryDays");
    run(Files.readString(Path.of(args[0])),"FishingUtils.lua");
    Field r=RandAbstract.class.getDeclaredField("rand"); r.setAccessible(true); r.set(RandStandard.INSTANCE,new CellularAutomatonRNG(new byte[]{0x12,0x34,0x56,0x78}));
    FishSchoolManager manager=FishSchoolManager.getInstance(); Field fish=FishSchoolManager.class.getDeclaredField("seed"); fish.setAccessible(true); fish.setInt(manager,12345);
    Field trash=FishSchoolManager.class.getDeclaredField("trashSeed"); trash.setAccessible(true); trash.setInt(manager,54321);
    for(int i=0;i<4;i++){LuaEventManager.triggerEvent("EveryDays"); System.out.printf("DAILY %d=%d,%d%n",i+1,fish.getInt(manager),trash.getInt(manager));}
  }
}
