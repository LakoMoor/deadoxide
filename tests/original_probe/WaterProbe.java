import java.lang.reflect.*;
// Exhaustive synthetic cardinal cases for water, solidtrans, solidfloor on both squares.
public class WaterProbe {
 public static void main(String[] args) throws Exception {
  Class<?> sq=Class.forName("zombie.iso.IsoGridSquare"), lookup=Class.forName("zombie.iso.IsoGridSquare$GetSquare"), flag=Class.forName("zombie.iso.SpriteDetails.IsoFlagType"), props=Class.forName("zombie.core.properties.PropertyContainer");
  var ctor=sq.getConstructor(Class.forName("zombie.iso.IsoCell"),Class.forName("zombie.iso.SliceY"),int.class,int.class,int.class);
  var calc=sq.getMethod("CalculateCollide",sq,boolean.class,boolean.class,boolean.class,boolean.class,lookup);
  var getProps=sq.getMethod("getProperties"); var set=props.getMethod("set",flag);
  Object[] flags={flag.getField("water").get(null),flag.getField("solidtrans").get(null),flag.getField("solidfloor").get(null)};
  for(int d=0;d<4;d++){
   int dx=d==0?1:d==1?-1:0, dy=d==2?1:d==3?-1:0;
   StringBuilder bits=new StringBuilder();
   for(int m=0;m<64;m++){
    Object a=ctor.newInstance(null,null,0,0,0),b=ctor.newInstance(null,null,dx,dy,0);
    for(int n=0;n<3;n++) {if((m&(1<<n))!=0)set.invoke(getProps.invoke(a),flags[n]); if((m&(1<<(n+3)))!=0)set.invoke(getProps.invoke(b),flags[n]);}
    Object get=Proxy.newProxyInstance(lookup.getClassLoader(),new Class<?>[]{lookup},(p,method,c)->{
      int x=(int)c[0],y=(int)c[1],z=(int)c[2]; return z!=0?null:x==0&&y==0?a:x==dx&&y==dy?b:null;
    });
    boolean blocked=(boolean)calc.invoke(a,b,false,false,false,false,get); bits.append(blocked?'1':'0');
   }
   System.out.println(dx+","+dy+":"+bits);
  }
 }
}
