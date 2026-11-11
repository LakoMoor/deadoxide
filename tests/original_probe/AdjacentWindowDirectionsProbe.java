import java.lang.reflect.*;
public class AdjacentWindowDirectionsProbe {
 public static void main(String[] args) throws Exception {
  Class<?> sq=Class.forName("zombie.iso.IsoGridSquare"), lookup=Class.forName("zombie.iso.IsoGridSquare$GetSquare"), flag=Class.forName("zombie.iso.SpriteDetails.IsoFlagType"), props=Class.forName("zombie.core.properties.PropertyContainer");
  var ctor=sq.getConstructor(Class.forName("zombie.iso.IsoCell"),Class.forName("zombie.iso.SliceY"),int.class,int.class,int.class);
  var calc=sq.getMethod("CalculateCollide",sq,boolean.class,boolean.class,boolean.class,boolean.class,lookup);
  var getProps=sq.getMethod("getProperties"); var set=props.getMethod("set",flag);
  String[] names={"solidtrans","windowN","windowW","WindowN","WindowW","HoppableN","HoppableW"};
  for(int sy=-1;sy<=1;sy++)for(int sx=-1;sx<=1;sx++){ if(sx==0&&sy==0)continue;
  for(int y=-1;y<=1;y++)for(int x=-1;x<=1;x++)for(int mask=0;mask<128;mask++){
   Object[][] grid=new Object[5][5]; for(int j=0;j<5;j++)for(int i=0;i<5;i++)grid[j][i]=ctor.newInstance(null,null,i-2,j-2,0);
   Object source=grid[sy+2][sx+2], dest=grid[2][2]; set.invoke(getProps.invoke(dest),flag.getField("solidtrans").get(null));
   for(int n=0;n<names.length;n++)if((mask&(1<<n))!=0)set.invoke(getProps.invoke(grid[y+2][x+2]),flag.getField(names[n]).get(null));
   Object get=Proxy.newProxyInstance(lookup.getClassLoader(),new Class<?>[]{lookup},(p,m,c)->{int xx=(int)c[0],yy=(int)c[1],z=(int)c[2];return z==0&&xx>=-2&&xx<=2&&yy>=-2&&yy<=2?grid[yy+2][xx+2]:null;});
   boolean blocked=(boolean)calc.invoke(source,dest,false,false,false,false,get);
   System.out.println(sx+","+sy+","+x+","+y+","+mask+":"+(blocked?1:0));
  }
 }
 }
}
