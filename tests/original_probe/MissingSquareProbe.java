import java.lang.reflect.*;
public class MissingSquareProbe {
 public static void main(String[] args) throws Exception {
  Class<?> sq=Class.forName("zombie.iso.IsoGridSquare"), lookup=Class.forName("zombie.iso.IsoGridSquare$GetSquare");
  var ctor=sq.getConstructor(Class.forName("zombie.iso.IsoCell"),Class.forName("zombie.iso.SliceY"),int.class,int.class,int.class);
  var calc=sq.getMethod("CalculateCollide",sq,boolean.class,boolean.class,boolean.class,boolean.class,lookup);
  for(int dy:new int[]{-1,1}) for(int dx:new int[]{-1,1}) for(int present=0;present<8;present++){
   Object a=ctor.newInstance(null,null,0,0,0), x=ctor.newInstance(null,null,dx,0,0), y=ctor.newInstance(null,null,0,dy,0), b=ctor.newInstance(null,null,dx,dy,0);
   final int mask=present;
   Object get=Proxy.newProxyInstance(lookup.getClassLoader(),new Class<?>[]{lookup},(p,m,c)->{
    int xx=(int)c[0], yy=(int)c[1], z=(int)c[2];
    if(z!=0)return null; if(xx==0&&yy==0)return a; if(xx==dx&&yy==0)return (mask&1)!=0?x:null; if(xx==0&&yy==dy)return (mask&2)!=0?y:null; if(xx==dx&&yy==dy)return (mask&4)!=0?b:null; return null;
   });
   try { Object v=calc.invoke(a,(present&4)!=0?b:null,false,false,false,false,get); System.out.println(dx+","+dy+" mask="+present+" => "+v); }
   catch(InvocationTargetException e){System.out.println(dx+","+dy+" mask="+present+" => ERROR "+e.getCause());}
  }
 }
}
