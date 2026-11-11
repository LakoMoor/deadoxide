import java.lang.reflect.*;
public class TransparentDiagonalProbe {
 public static void main(String[] args) throws Exception {
  Class<?> sq=Class.forName("zombie.iso.IsoGridSquare"), lookup=Class.forName("zombie.iso.IsoGridSquare$GetSquare"), flag=Class.forName("zombie.iso.SpriteDetails.IsoFlagType"), props=Class.forName("zombie.core.properties.PropertyContainer");
  var ctor=sq.getConstructor(Class.forName("zombie.iso.IsoCell"),Class.forName("zombie.iso.SliceY"),int.class,int.class,int.class);
  var calc=sq.getMethod("CalculateCollide",sq,boolean.class,boolean.class,boolean.class,boolean.class,lookup);
  var getProps=sq.getMethod("getProperties"); var clear=props.getMethod("Clear"); var set=props.getMethod("set",flag);
  Object[] flags={flag.getField("solidtrans").get(null),flag.getField("collideN").get(null),flag.getField("collideW").get(null)};
  for(int dy:new int[]{-1,1})for(int dx:new int[]{-1,1}){
   Object[] s={ctor.newInstance(null,null,0,0,0),ctor.newInstance(null,null,dx,0,0),ctor.newInstance(null,null,0,dy,0),ctor.newInstance(null,null,dx,dy,0)};
   Object get=Proxy.newProxyInstance(lookup.getClassLoader(),new Class<?>[]{lookup},(p,m,c)->{int x=(int)c[0],y=(int)c[1],z=(int)c[2];return z!=0?null:x==0&&y==0?s[0]:x==dx&&y==0?s[1]:x==0&&y==dy?s[2]:x==dx&&y==dy?s[3]:null;});
   StringBuilder bits=new StringBuilder();
   for(int mask=0;mask<4096;mask++){
    for(int i=0;i<4;i++){Object container=getProps.invoke(s[i]);clear.invoke(container);for(int bit=0;bit<3;bit++)if((mask&(1<<(i*3+bit)))!=0)set.invoke(container,flags[bit]);}
    boolean blocked=(boolean)calc.invoke(s[0],s[3],false,false,false,false,get);bits.append(blocked?'1':'0');
   }
   System.out.println(dx+","+dy+":"+bits);
  }
 }
}
