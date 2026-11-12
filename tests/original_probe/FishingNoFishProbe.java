import java.nio.file.*;
import java.lang.reflect.*;
import java.util.*;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.core.random.RandStandard;
import zombie.iso.FishSchoolManager;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import se.krka.kahlua.vm.KahluaTable;
public class FishingNoFishProbe {
 static void run(String s) throws Exception {Object[] r=LuaManager.thread.pcall(LuaCompiler.loadstring(s,"zones",LuaManager.env),new Object[0]);if(!Boolean.TRUE.equals(r[0]))throw new IllegalStateException(Arrays.toString(r));}
 public static void main(String[] a) throws Exception {
  ZomboidFileSystem.instance.setCacheDir("/tmp/pz-nofish-probe");ZomboidFileSystem.instance.init();RandStandard.INSTANCE.init();LuaManager.init();
  run(Files.readString(Path.of(a[0])));
  KahluaTable table=(KahluaTable)LuaManager.getTableObject("Fishing.NoFishZones");
  System.out.println("ZONE size="+table.size());
  FishSchoolManager f=FishSchoolManager.getInstance();
  Method method=FishSchoolManager.class.getDeclaredMethod("isNoFishZone",int.class,int.class);method.setAccessible(true);
  for(int i=1;i<=table.len();i++) {
   KahluaTable z=(KahluaTable)table.rawget(i);int x=((Double)z.rawget("x1")).intValue();int y=((Double)z.rawget("y1")).intValue();
   System.out.printf("ZONE %d,%d,%d=%s%n",i,x,y,method.invoke(f,x,y));
  }
  for(int[] p:new int[][]{{0,0},{6350,5243},{6363,5258},{6364,5258},{13414,1854}})System.out.printf("POINT %d,%d=%s%n",p[0],p[1],method.invoke(f,p[0],p[1]));
 }
}
