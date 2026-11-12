import java.lang.reflect.Field;
import zombie.iso.FishSchoolManager;
public class FishingTrashProbe {
    public static void main(String[] args) throws Exception {
        FishSchoolManager manager = FishSchoolManager.getInstance();
        Field seed = FishSchoolManager.class.getDeclaredField("trashSeed");
        seed.setAccessible(true);
        int[] seeds = {0, 12345, 99999, -1, Integer.MAX_VALUE};
        int[][] points = {{0,0},{8306,11692},{-20,40},{300000,-400000},{2147483600,-2147483600},{100,100},{200,200},{300,300},{400,400},{500,500},{600,600},{700,700},{800,800},{900,900},{1000,1000}};
        for (int s : seeds) {
            seed.setInt(manager, s);
            for (int[] p : points) {
                double value = manager.getTrashAbundance(p[0], p[1]);
                System.out.printf("TRASH %d,%d,%d=%016x%n", s,p[0],p[1],Double.doubleToRawLongBits(value));
            }
        }
    }
}
