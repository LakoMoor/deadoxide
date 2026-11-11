import java.lang.reflect.*;
import java.util.*;

// Headless synthetic squares, no world or profile; coordinates and flags are explicit.
public class CollisionProbe {
  public static void main(String[] args) throws Exception {
    Class<?> square = Class.forName("zombie.iso.IsoGridSquare");
    Class<?> cell = Class.forName("zombie.iso.IsoCell");
    Class<?> slice = Class.forName("zombie.iso.SliceY");
    Class<?> flag = Class.forName("zombie.iso.SpriteDetails.IsoFlagType");
    Class<?> getSquare = Class.forName("zombie.iso.IsoGridSquare$GetSquare");
    var create = square.getConstructor(cell, slice, int.class, int.class, int.class);
    var properties = square.getMethod("getProperties");
    var set = Class.forName("zombie.core.properties.PropertyContainer").getMethod("set", flag);
    var collision = square.getMethod("CalculateCollide", square, boolean.class, boolean.class, boolean.class, boolean.class, getSquare);
    String[][] cases = {{}, {"solid"}, {"solidtrans"}, {"collideW"}, {"collideN"}, {"WallW"}, {"WallN"},
       {"doorW"}, {"windowW"}, {"collideW", "doorW"}, {"collideW", "windowW"},
       {"WallW", "collideW"}, {"WallN", "collideN"}};
    for (int direction = 0; direction < 2; direction++) {
      int bx = direction == 0 ? 1 : 0, by = direction == 0 ? 0 : 1;
      for (String[] flags : cases) {
        Object a = create.newInstance(null, null, 0, 0, 0);
        Object b = create.newInstance(null, null, bx, by, 0);
        for (String name : flags) set.invoke(properties.invoke(b), flag.getField(name).get(null));
        Object lookup = Proxy.newProxyInstance(getSquare.getClassLoader(), new Class<?>[]{getSquare}, (proxy, method, coords) -> {
          int x = (int) coords[0], y = (int) coords[1], z = (int) coords[2];
          return z != 0 ? null : x == 0 && y == 0 ? a : x == bx && y == by ? b : null;
        });
        try {
          Object f = collision.invoke(a, b, false, false, false, false, lookup);
          Object r = collision.invoke(b, a, false, false, false, false, lookup);
          System.out.println((direction == 0 ? "east" : "south") + " " + Arrays.toString(flags) + ": " + f + ", " + r);
        } catch (InvocationTargetException ex) { System.out.println("ERROR " + ex.getCause()); }
      }
    }
  }
}
