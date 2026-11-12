import java.lang.reflect.Field;
import java.lang.reflect.Method;
import gnu.trove.map.hash.TLongIntHashMap;
import java.util.ArrayList;
import java.util.GregorianCalendar;
import java.util.TimeZone;
import gnu.trove.map.hash.TLongObjectHashMap;
import zombie.GameTime;
import zombie.iso.FishSchoolManager;
import zombie.util.PZCalendar;
public class FishingChumProbe {
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        GameTime time=new GameTime(); GameTime.setInstance(time);
        GregorianCalendar calendar=new GregorianCalendar(TimeZone.getTimeZone("UTC")); calendar.clear();
        time.setCalender(new PZCalendar(calendar)); time.setYear(1993); time.setMonth(6); time.setDay(8); time.setTimeOfDay(10.0f);
        FishSchoolManager fish=FishSchoolManager.getInstance();
        Field noFish=FishSchoolManager.class.getDeclaredField("noFishZones"); noFish.setAccessible(true);
        ((ArrayList<int[]>)noFish.get(fish)).add(new int[]{-1000,-1000,1000,1000});
        Field ch=FishSchoolManager.class.getDeclaredField("chumPoints"); ch.setAccessible(true);
        TLongObjectHashMap<FishSchoolManager.ChumData> chum=(TLongObjectHashMap<FishSchoolManager.ChumData>)ch.get(null); chum.clear();
        fish.addChum(100,100,200);
        int[][] points={{100,100},{103,100},{104,100},{104,102},{105,100},{100,106}};
        float[] times={10f,10.5f,11f,11.666667f,12f,15f,15.016667f};
        for(float t:times){time.setTimeOfDay(t); for(int[] p:points) System.out.printf("CHUM %.6f,%d,%d=%.1f%n",t,p[0],p[1],fish.getFishAbundance(p[0],p[1]));}
        time.setTimeOfDay(10f); fish.addChum(104,100,200); time.setTimeOfDay(11f);
        System.out.printf("CHUM combined=%.1f%n",fish.getFishAbundance(102,100));
        time.setTimeOfDay(10f); fish.addChum(104,100,200); time.setTimeOfDay(11f);
        System.out.printf("CHUM replaced=%.1f%n",fish.getFishAbundance(102,100));
        chum.clear(); time.setTimeOfDay(10f); fish.addChum(100,100,200);
        Method hash=FishSchoolManager.class.getDeclaredMethod("coordsToHash",int.class,int.class); hash.setAccessible(true);
        int[][] hashes={{100,100},{-1,2},{1,-2},{-7,-9},{2147483647,-2147483648}};
        for(int[] p:hashes) System.out.printf("HASH %d,%d=%d%n",p[0],p[1],(Long)hash.invoke(fish,p[0],p[1]));
        Field noiseField=FishSchoolManager.class.getDeclaredField("noiseFishPointDisabler"); noiseField.setAccessible(true);
        TLongIntHashMap noise=(TLongIntHashMap)noiseField.get(null); time.setTimeOfDay(11f);
        noise.put((Long)hash.invoke(fish,100,100), 0);
        System.out.printf("CHUM noise=%.1f%n",fish.getFishAbundance(100,100)); noise.clear();
        time.setTimeOfDay(15f); fish.updateFishingData();
        System.out.printf("CHUM expiry_equal=%.1f%n",fish.getFishAbundance(100,100));
        time.setTimeOfDay(15.016667f); fish.updateFishingData();
        System.out.printf("CHUM expiry_after=%.1f%n",fish.getFishAbundance(100,100));
    }
}
