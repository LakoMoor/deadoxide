import java.lang.reflect.*;

// Enumerate static flags on the four squares around each diagonal crossing.
public class DiagonalProbe {
  public static void main(String[] args) throws Exception {
    Class<?> sq = Class.forName("zombie.iso.IsoGridSquare");
    Class<?> flags = Class.forName("zombie.iso.SpriteDetails.IsoFlagType");
    Class<?> props = Class.forName("zombie.core.properties.PropertyContainer");
    Class<?> lookup = Class.forName("zombie.iso.IsoGridSquare$GetSquare");
    var constructor = sq.getConstructor(Class.forName("zombie.iso.IsoCell"), Class.forName("zombie.iso.SliceY"), int.class, int.class, int.class);
    var getProps = sq.getMethod("getProperties");
    var clear = props.getMethod("Clear");
    var set = props.getMethod("set", flags);
    var calc = sq.getMethod("CalculateCollide", sq, boolean.class, boolean.class, boolean.class, boolean.class, lookup);
    Object[] flagValues = {flags.getField("solid").get(null), flags.getField("collideN").get(null), flags.getField("collideW").get(null)};
    for (int dy : new int[]{-1, 1}) for (int dx : new int[]{-1, 1}) {
      Object[] squares = {constructor.newInstance(null,null,0,0,0),constructor.newInstance(null,null,dx,0,0),constructor.newInstance(null,null,0,dy,0),constructor.newInstance(null,null,dx,dy,0)};
      Object get = Proxy.newProxyInstance(lookup.getClassLoader(), new Class<?>[]{lookup}, (p,m,c) -> {
        int x=(int)c[0], y=(int)c[1], z=(int)c[2];
        if (z!=0) return null;
        if (x==0 && y==0) return squares[0];
        if (x==dx && y==0) return squares[1];
        if (x==0 && y==dy) return squares[2];
        if (x==dx && y==dy) return squares[3];
        return null;
      });
      StringBuilder bits = new StringBuilder(4096);
      for (int mask=0;mask<4096;mask++) {
        for (int i=0;i<4;i++) {
          Object container=getProps.invoke(squares[i]); clear.invoke(container);
          for (int bit=0;bit<3;bit++) if ((mask & (1 << (i*3+bit)))!=0) set.invoke(container,flagValues[bit]);
        }
        boolean blocked=(boolean)calc.invoke(squares[0],squares[3],false,false,false,false,get);
        bits.append(blocked ? '1' : '0');
      }
      System.out.println(dx+","+dy+":"+bits);
    }
  }
}
