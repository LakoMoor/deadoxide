import java.nio.file.*;
import java.lang.reflect.*;
import java.util.*;
import zombie.ZomboidFileSystem;
import zombie.GameTime;
import zombie.Lua.LuaManager;
import zombie.core.random.RandStandard;
import zombie.iso.FishSchoolManager;
import zombie.util.PZCalendar;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import gnu.trove.map.hash.TLongObjectHashMap;
public class FishingAbundanceProbe {
 static void run(String s)throws Exception{Object[] r=LuaManager.thread.pcall(LuaCompiler.loadstring(s,"zones",LuaManager.env),new Object[0]);if(!Boolean.TRUE.equals(r[0]))throw new IllegalStateException(Arrays.toString(r));}
 @SuppressWarnings("unchecked") public static void main(String[] a)throws Exception {
  ZomboidFileSystem.instance.setCacheDir("/tmp/pz-abundance-probe");ZomboidFileSystem.instance.init();RandStandard.INSTANCE.init();LuaManager.init();run(Files.readString(Path.of(a[0])));
  GameTime game=new GameTime();GameTime.setInstance(game);GregorianCalendar cal=new GregorianCalendar(TimeZone.getTimeZone("UTC"));cal.clear();game.setCalender(new PZCalendar(cal));game.setYear(1993);game.setMonth(6);game.setDay(8);
  FishSchoolManager f=FishSchoolManager.getInstance();Field seed=FishSchoolManager.class.getDeclaredField("seed");seed.setAccessible(true);
  Field ch=FishSchoolManager.class.getDeclaredField("chumPoints");ch.setAccessible(true);TLongObjectHashMap<FishSchoolManager.ChumData> chum=(TLongObjectHashMap<FishSchoolManager.ChumData>)ch.get(null);
  Method point=FishSchoolManager.class.getDeclaredMethod("isFishPoint",int.class,int.class);point.setAccessible(true);Method rad=FishSchoolManager.class.getDeclaredMethod("getFishPointRadius",int.class,int.class);rad.setAccessible(true);
  int[][] positions={{6357,5250},{0,0},{8306,11692},{13419,1859},{365,9715},{100,100},{200,200},{300,300},{400,400},{500,500},{600,600},{700,700},{800,800},{900,900}};
  for(int s:new int[]{1485,12345})for(int[] p:positions){seed.setInt(f,s);int active=0;for(int dx=-6;dx<=6;dx++)for(int dy=-6;dy<=6;dy++){int x=p[0]+dx,y=p[1]+dy;if((Boolean)point.invoke(f,x,y)){float r=(Float)rad.invoke(f,x,y);double d=Math.sqrt((float)(dx*dx+dy*dy));if(d<=r+2.0f+1.5)active++;}}
   chum.clear();game.setTimeOfDay(10f);f.addChum(p[0],p[1],200);game.setTimeOfDay(11f);
   if(active==0)System.out.printf("ABUND %d,%d,%d=%.1f%n",s,p[0],p[1],f.getFishAbundance(p[0],p[1]));else System.out.printf("UNSUPPORTED %d,%d,%d=%d%n",s,p[0],p[1],active);
  }
 }
}
