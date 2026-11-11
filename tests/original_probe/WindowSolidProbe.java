import java.lang.reflect.*;
public class WindowSolidProbe {
 public static void main(String[] args) throws Exception {
  Class<?> sq=Class.forName("zombie.iso.IsoGridSquare"), lookup=Class.forName("zombie.iso.IsoGridSquare$GetSquare"), flag=Class.forName("zombie.iso.SpriteDetails.IsoFlagType"), props=Class.forName("zombie.core.properties.PropertyContainer");
  var ctor=sq.getConstructor(Class.forName("zombie.iso.IsoCell"),Class.forName("zombie.iso.SliceY"),int.class,int.class,int.class);
  var calc=sq.getMethod("CalculateCollide",sq,boolean.class,boolean.class,boolean.class,boolean.class,lookup);
  var getProps=sq.getMethod("getProperties"); var set=props.getMethod("set",flag);
  String[] names={"solidtrans","windowN","windowW","WindowN","WindowW","collideN","collideW","transparentN","transparentW"};
  for(int dy:new int[]{-1,0,1})for(int dx:new int[]{-1,0,1}) { if(dx==0&&dy==0)continue;
   StringBuilder bits=new StringBuilder(); for(int mask=0;mask<512;mask++){
    Object a=ctor.newInstance(null,null,0,0,0),b=ctor.newInstance(null,null,dx,dy,0),x=ctor.newInstance(null,null,dx,0,0),y=ctor.newInstance(null,null,0,dy,0);
    for(int i=0;i<names.length;i++)if((mask&(1<<i))!=0)set.invoke(getProps.invoke(b),flag.getField(names[i]).get(null));
    Object get=Proxy.newProxyInstance(lookup.getClassLoader(),new Class<?>[]{lookup},(p,m,c)->{int xx=(int)c[0],yy=(int)c[1],z=(int)c[2];return z!=0?null:xx==0&&yy==0?a:xx==dx&&yy==dy?b:xx==dx&&yy==0?x:xx==0&&yy==dy?y:null;});
    boolean actual=(boolean)calc.invoke(a,b,false,false,false,false,get);
    bits.append(actual ? '1' : '0');

   }
   System.out.println(dx+","+dy+":"+bits);
  }
 }
}
