-- unit.fighter@modern: Fighter (mesh.fighter_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fighter_modern','Fighter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fighter_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.olive_drab',0,'','{"y":2.7,"z":-0.1,"w":0.3,"d":1.9,"h":0.32,"taper":0.3,"bank":18}'),
  ('cowling','cylinder','mat.cloth',1,'fuselage','{"y":0.16,"z":0.78,"w":0.34,"h":0.36,"segments":8,"pitch":90}'),
  ('nose_band','cylinder','mat.cloth',1,'fuselage','{"y":0.16,"z":1.06,"w":0.35,"h":0.1,"segments":8,"pitch":90}'),
  ('spinner','cone','mat.cast_iron',0,'fuselage','{"y":0.16,"z":1.16,"w":0.18,"h":0.2,"segments":6,"pitch":90}'),
  ('propeller','propeller','mat.cast_iron',0,'fuselage','{"y":0.16,"z":1.2,"w":1,"blades":3,"chord":0.09}'),
  ('wing_e','box','mat.olive_drab',0,'fuselage','{"y":0.05,"z":0.3,"w":0.06,"d":0.6,"h":1.3,"taper":0.5,"roll":90}'),
  ('wing_w','box','mat.olive_drab',0,'fuselage','{"y":0.05,"z":0.3,"w":0.06,"d":0.6,"h":1.3,"taper":0.5,"roll":-90}'),
  ('wing_tips','box','mat.cloth',1,'fuselage','{"x":-1.3,"y":0.06,"z":0.3,"w":0.24,"d":0.34,"h":0.04,"repeat":2,"dx":2.6}'),
  ('canopy','box','mat.glass',0,'fuselage','{"y":0.3,"z":0,"w":0.2,"d":0.42,"h":0.14,"taper":0.45}'),
  ('tailplane','box','mat.olive_drab',0,'fuselage','{"y":0.16,"z":-0.85,"w":0.9,"d":0.26,"h":0.03}'),
  ('fin','box','mat.cloth',1,'fuselage','{"y":0.2,"z":-0.85,"w":0.03,"d":0.34,"h":0.36,"taper":0.45}'),
  ('guns','cylinder','mat.cast_iron',0,'fuselage','{"x":-0.6,"y":0.06,"z":0.55,"w":0.035,"h":0.18,"segments":4,"pitch":90,"repeat":2,"dx":1.2,"far":0}'),
  ('spark','cone','mat.flash',0,'guns','{"y":0.18,"w":0.07,"h":0.07,"segments":3,"flash":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.24,"w":0.2,"h":0.24,"segments":5,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fighter_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.fighter'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.fighter_modern'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
