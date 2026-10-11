namespace N {
  export import A = B.C;
  export { x } from "m";
}
declare module "mod" {
  export * from "m";
  export { a } from "n";
  export import Z = Q.R;
}
declare global { }
