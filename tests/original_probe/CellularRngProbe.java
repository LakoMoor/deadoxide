import org.uncommons.maths.random.CellularAutomatonRNG;
public class CellularRngProbe {
  public static void main(String[] args) {
    int[] seeds={0,1,-1,0x12345678,0x80000000};
    int[] bounds={1,2,3,16,100000,1073741825,2147483647};
    for(int seed:seeds){
      byte[] b={(byte)(seed>>>24),(byte)(seed>>>16),(byte)(seed>>>8),(byte)seed};
      CellularAutomatonRNG rng=new CellularAutomatonRNG(b);
      for(int i=0;i<14;i++){int bound=bounds[i%bounds.length]; System.out.printf("RNG %08x,%d,%d=%d%n",seed,i,bound,rng.nextInt(bound));}
    }
  }
}
