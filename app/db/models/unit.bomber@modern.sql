-- unit.bomber@modern: Bomber (mesh.bomber_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bomber_modern','Bomber');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bomber_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.olive_drab',0,'','{"y":2.9,"z":-0.1,"w":0.42,"d":2.8,"h":0.44,"taper":0.2,"bank":8}'),
  ('nose','cone','mat.glass',0,'fuselage','{"y":0.21,"z":1.3,"w":0.4,"h":0.34,"segments":8,"pitch":90}'),
  ('cockpit','box','mat.glass',0,'fuselage','{"y":0.4,"z":0.85,"w":0.3,"d":0.36,"h":0.12,"taper":0.4}'),
  ('wing_e','box','mat.olive_drab',0,'fuselage','{"y":0.18,"z":0.25,"w":0.08,"d":0.72,"h":2,"taper":0.5,"roll":90}'),
  ('wing_w','box','mat.olive_drab',0,'fuselage','{"y":0.18,"z":0.25,"w":0.08,"d":0.72,"h":2,"taper":0.5,"roll":-90}'),
  ('nacelles','cylinder','mat.olive_drab',0,'fuselage','{"x":-1.15,"y":0.18,"z":0.1,"w":0.2,"h":0.62,"segments":6,"pitch":90,"repeat":2,"dx":0.5}'),
  ('nacelles_e','cylinder','mat.olive_drab',0,'fuselage','{"x":0.65,"y":0.18,"z":0.1,"w":0.2,"h":0.62,"segments":6,"pitch":90,"repeat":2,"dx":0.5}'),
  ('prop_1','propeller','mat.cast_iron',0,'fuselage','{"x":-1.15,"y":0.18,"z":0.74,"w":0.7,"blades":3,"chord":0.07,"far":0}'),
  ('prop_2','propeller','mat.cast_iron',0,'fuselage','{"x":-0.65,"y":0.18,"z":0.74,"w":0.7,"blades":3,"chord":0.07,"far":0}'),
  ('prop_3','propeller','mat.cast_iron',0,'fuselage','{"x":0.65,"y":0.18,"z":0.74,"w":0.7,"blades":3,"chord":0.07,"far":0}'),
  ('prop_4','propeller','mat.cast_iron',0,'fuselage','{"x":1.15,"y":0.18,"z":0.74,"w":0.7,"blades":3,"chord":0.07,"far":0}'),
  ('band','box','mat.cloth',1,'fuselage','{"y":-0.01,"z":-0.55,"w":0.44,"d":0.3,"h":0.46,"taper":0.2}'),
  ('wing_tips','box','mat.cloth',1,'fuselage','{"x":-1.85,"y":0.2,"z":0.25,"w":0.3,"d":0.4,"h":0.05,"repeat":2,"dx":3.7}'),
  ('tailplane','box','mat.cloth',1,'fuselage','{"y":0.24,"z":-1.32,"w":1.5,"d":0.34,"h":0.04}'),
  ('fin','box','mat.cloth',1,'fuselage','{"y":0.3,"z":-1.3,"w":0.04,"d":0.5,"h":0.62,"taper":0.4}'),
  ('turret','dome','mat.glass',0,'fuselage','{"y":0.43,"z":0.45,"w":0.24,"h":0.12,"segments":7,"far":0}'),
  ('turret_guns','cylinder','mat.cast_iron',0,'turret','{"x":-0.04,"y":0.06,"z":0.06,"w":0.03,"h":0.28,"segments":4,"pitch":80,"repeat":2,"dx":0.08,"far":0}'),
  ('spark','cone','mat.flash',0,'turret_guns','{"y":0.28,"w":0.06,"h":0.06,"segments":3,"flash":3,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.2,"w":0.18,"h":0.2,"segments":5,"pitch":180,"far":0}'),
  ('bay','box','mat.cast_iron',0,'fuselage','{"y":-0.02,"z":0.15,"w":0.26,"d":0.8,"h":0.02,"far":0}'),
  ('bomb_drop','box','mat.flash',0,'fuselage','{"y":-0.02,"z":0.15,"w":0.02,"d":0.02,"h":0.02,"flash":1,"far":0}'),
  ('bombs','cylinder','mat.cast_iron',0,'bomb_drop','{"y":-0.5,"z":-0.25,"w":0.1,"h":0.26,"segments":5,"pitch":90,"repeat":3,"dz":0.25,"far":0}'),
  ('bombs_low','cylinder','mat.cast_iron',0,'bomb_drop','{"y":-1.2,"z":-0.15,"w":0.1,"h":0.26,"segments":5,"pitch":90,"repeat":2,"dz":0.3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bomber_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.bomber'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.bomber_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
