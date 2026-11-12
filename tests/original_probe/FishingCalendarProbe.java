import java.util.GregorianCalendar;
import java.util.TimeZone;
import zombie.GameTime;
import zombie.util.PZCalendar;
public class FishingCalendarProbe {
  public static void main(String[] args) {
    int[][] dates={{1993,6,8},{1993,6,0},{2000,1,29},{2000,2,0},{1970,0,1},{1969,11,31},{2024,1,29},{2100,2,0}};
    float[] times={0f,7.5f,11.42f,23.99998f};
    GameTime game=new GameTime();
    for(int[] d:dates) for(float t:times) {
      GregorianCalendar cal=new GregorianCalendar(TimeZone.getTimeZone("UTC")); cal.clear();
      game.setCalender(new PZCalendar(cal)); game.setYear(d[0]); game.setMonth(d[1]); game.setDay(d[2]); game.setTimeOfDay(t);
      System.out.printf("CAL %d,%d,%d,%08x=%d%n",d[0],d[1],d[2],Float.floatToRawIntBits(t),game.getCalender().getTimeInMillis()/60000L);
    }
  }
}
